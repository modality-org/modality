//! The MOD contract's emission schedule, written against the raw Modality
//! host ABI: the host calls `alloc(len)`, writes the input JSON there, calls
//! `execute(ptr, len)`, and reads a little-endian `u32` length followed by the
//! output JSON at the returned pointer.
//!
//! One operation, `mint`: pay the subsidy of each named miner block, in index
//! order, starting at the first block not yet paid. The parameters are the
//! contract's genesis posts under `/network/emission`, which its rules never
//! let change. The program pays nothing past the emission cap.
//!
//! See `docs/concepts/mod-contract.md` for the rules this program runs under.

use serde_json::{json, Map, Value};

pub const ASSET: &str = "MOD";

pub const BLOCK_SUBSIDY: &str = "/network/emission/block_subsidy.num";
pub const HALVING_INTERVAL: &str = "/network/emission/halving_interval.num";
pub const SLOW_START: &str = "/network/emission/slow_start.num";
pub const CAP: &str = "/network/emission/cap.num";
pub const NEXT_INDEX: &str = "/emission/next_index.num";
pub const EMITTED: &str = "/emission/emitted.num";

#[no_mangle]
pub extern "C" fn alloc(len: i32) -> i32 {
    let mut buf = Vec::<u8>::with_capacity(len.max(0) as usize);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr as i32
}

/// # Safety
/// `ptr` and `len` must describe bytes the host wrote after calling `alloc`.
#[no_mangle]
pub unsafe extern "C" fn execute(ptr: i32, len: i32) -> i32 {
    let input = core::slice::from_raw_parts(ptr as *const u8, len.max(0) as usize);
    let out = serde_json::to_vec(&respond(input)).unwrap_or_default();
    let mut buf = Vec::with_capacity(4 + out.len());
    buf.extend_from_slice(&(out.len() as u32).to_le_bytes());
    buf.extend_from_slice(&out);
    let ptr = buf.as_ptr();
    core::mem::forget(buf);
    ptr as i32
}

/// The host's output object for one input, whether the operation ran or not.
pub fn respond(input: &[u8]) -> Value {
    match serde_json::from_slice::<Value>(input)
        .map_err(|e| format!("input is not JSON: {e}"))
        .and_then(|input| run(&input))
    {
        Ok(actions) => json!({"actions": actions, "gas_used": 0, "errors": []}),
        Err(error) => json!({"actions": [], "gas_used": 0, "errors": [error]}),
    }
}

/// The emission parameters the genesis posted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub block_subsidy: u64,
    pub halving_interval: u64,
    pub slow_start: u64,
    pub cap: u64,
}

impl Schedule {
    fn read(state: &Map<String, Value>) -> Result<Schedule, String> {
        Ok(Schedule {
            block_subsidy: whole(state, BLOCK_SUBSIDY)?,
            halving_interval: whole(state, HALVING_INTERVAL)?,
            slow_start: whole(state, SLOW_START)?,
            cap: whole(state, CAP)?,
        })
    }

    /// The subsidy of the miner block at `index`, before the cap. Block 0
    /// pays nothing. The subsidy halves every `halving_interval` blocks
    /// (never, when 0), and over the first `slow_start` blocks rises in a
    /// straight line to the full amount.
    pub fn subsidy(&self, index: u64) -> u64 {
        if index == 0 || self.block_subsidy == 0 {
            return 0;
        }
        let halvings = match self.halving_interval {
            0 => 0,
            h => (index - 1) / h,
        };
        if halvings >= 64 {
            return 0;
        }
        let full = self.block_subsidy >> halvings;
        if index < self.slow_start {
            return (full as u128 * index as u128 / self.slow_start as u128) as u64;
        }
        full
    }
}

fn run(input: &Value) -> Result<Vec<Value>, String> {
    let args = input.get("args").ok_or("input has no args")?;
    let state = input
        .pointer("/context/state")
        .and_then(Value::as_object)
        .ok_or("input has no context.state")?;
    match args.get("op").and_then(Value::as_str) {
        Some("mint") => mint(state, args),
        Some(other) => Err(format!("unknown op {other:?}; use mint")),
        None => Err("args need an op".into()),
    }
}

/// `{"op": "mint", "blocks": [{"index": n, "to": "<contract id>"}, ...]}`:
/// the blocks must run on from `/emission/next_index.num` without a gap.
fn mint(state: &Map<String, Value>, args: &Value) -> Result<Vec<Value>, String> {
    let schedule = Schedule::read(state)?;
    let blocks = args
        .get("blocks")
        .and_then(Value::as_array)
        .filter(|b| !b.is_empty())
        .ok_or("mint needs blocks: the miner blocks it pays, in index order")?;
    let mut next = match whole(state, NEXT_INDEX)? {
        0 => 1,
        n => n,
    };
    let mut emitted = whole(state, EMITTED)?;
    let mut out = Vec::new();
    for block in blocks {
        let index = block
            .get("index")
            .and_then(Value::as_u64)
            .ok_or("each block needs a whole index")?;
        if index != next {
            return Err(format!("block {index} is out of turn; the next block to pay is {next}"));
        }
        let to = block
            .get("to")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("block {index} needs to: the contract it pays"))?;
        let amount = schedule.subsidy(index).min(schedule.cap.saturating_sub(emitted));
        if amount > 0 {
            out.push(send(to, amount));
            emitted += amount;
        }
        next = index.checked_add(1).ok_or("block index overflow")?;
    }
    out.push(post(NEXT_INDEX, next));
    out.push(post(EMITTED, emitted));
    Ok(out)
}

fn whole(state: &Map<String, Value>, path: &str) -> Result<u64, String> {
    match state.get(path) {
        None => Ok(0),
        Some(Value::Number(n)) => n.as_u64().ok_or_else(|| format!("{path} must be a whole number")),
        Some(Value::String(s)) => s.parse().map_err(|_| format!("{path} must be a whole number")),
        Some(_) => Err(format!("{path} must be a whole number")),
    }
}

fn post(path: &str, value: u64) -> Value {
    json!({"method": "post", "path": path, "value": value})
}

fn send(to: &str, amount: u64) -> Value {
    json!({"method": "send", "path": null, "value": {"asset_id": ASSET, "to_contract": to, "amount": amount}})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(next: u64, emitted: u64, cap: u64) -> Value {
        json!({
            BLOCK_SUBSIDY: "50", HALVING_INTERVAL: "4", SLOW_START: "0", CAP: cap.to_string(),
            NEXT_INDEX: next.to_string(), EMITTED: emitted.to_string(),
            "/__programs__/emission.wasm": "AGFzbQ==",
        })
    }

    fn call(blocks: Value, st: Value) -> Value {
        let input = json!({"args": {"op": "mint", "blocks": blocks}, "context": {"state": st}});
        respond(input.to_string().as_bytes())
    }

    fn blocks(from: u64, to: u64) -> Value {
        (from..=to).map(|i| json!({"index": i, "to": format!("W{i}")})).collect()
    }

    fn sent(out: &Value) -> Vec<(String, u64)> {
        assert_eq!(out["errors"], json!([]), "{out}");
        out["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["method"] == "send")
            .map(|a| (a["value"]["to_contract"].as_str().unwrap().to_string(), a["value"]["amount"].as_u64().unwrap()))
            .collect()
    }

    fn posted(out: &Value, path: &str) -> u64 {
        out["actions"].as_array().unwrap().iter().find(|a| a["path"] == path).unwrap()["value"].as_u64().unwrap()
    }

    fn error(out: Value) -> String {
        out["errors"][0].as_str().unwrap_or_default().to_string()
    }

    #[test]
    fn the_subsidy_halves_each_era() {
        let s = Schedule { block_subsidy: 50, halving_interval: 4, slow_start: 0, cap: 0 };
        let got: Vec<u64> = (0..=13).map(|i| s.subsidy(i)).collect();
        assert_eq!(got, vec![0, 50, 50, 50, 50, 25, 25, 25, 25, 12, 12, 12, 12, 6]);
        assert_eq!(Schedule { halving_interval: 0, ..s }.subsidy(1_000_000), 50);
        assert_eq!(Schedule { halving_interval: 1, ..s }.subsidy(70), 0);
    }

    #[test]
    fn the_slow_start_rises_to_the_full_subsidy() {
        let s = Schedule { block_subsidy: 100, halving_interval: 0, slow_start: 4, cap: 0 };
        let got: Vec<u64> = (0..=5).map(|i| s.subsidy(i)).collect();
        assert_eq!(got, vec![0, 25, 50, 75, 100, 100]);
    }

    #[test]
    fn a_mint_pays_each_block_and_moves_the_counters() {
        let out = call(blocks(1, 5), state(1, 0, 1_000));
        assert_eq!(sent(&out), vec![("W1".into(), 50), ("W2".into(), 50), ("W3".into(), 50), ("W4".into(), 50), ("W5".into(), 25)]);
        assert_eq!(posted(&out, NEXT_INDEX), 6);
        assert_eq!(posted(&out, EMITTED), 225);
        let fresh = call(blocks(1, 1), json!({BLOCK_SUBSIDY: "50", CAP: "100"}));
        assert_eq!(sent(&fresh), vec![("W1".into(), 50)]);
    }

    #[test]
    fn the_cap_stops_the_mint() {
        let out = call(blocks(3, 5), state(3, 80, 120));
        assert_eq!(sent(&out), vec![("W3".into(), 40)]);
        assert_eq!(posted(&out, NEXT_INDEX), 6);
        assert_eq!(posted(&out, EMITTED), 120);
    }

    #[test]
    fn blocks_are_paid_once_and_in_turn() {
        assert!(error(call(blocks(2, 3), state(1, 0, 1_000))).contains("next block to pay is 1"));
        assert!(error(call(blocks(1, 2), state(3, 100, 1_000))).contains("next block to pay is 3"));
        let gap = json!([{"index": 1, "to": "W"}, {"index": 3, "to": "W"}]);
        assert!(error(call(gap, state(1, 0, 1_000))).contains("block 3 is out of turn"));
        let twice = json!([{"index": 1, "to": "W"}, {"index": 1, "to": "W"}]);
        assert!(error(call(twice, state(1, 0, 1_000))).contains("out of turn"));
        assert!(error(call(json!([]), state(1, 0, 1_000))).contains("needs blocks"));
        assert!(error(call(json!([{"index": 1}]), state(1, 0, 1_000))).contains("needs to"));
    }
}
