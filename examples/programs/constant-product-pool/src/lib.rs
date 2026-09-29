//! A two-asset constant-product pool, written against the raw Modality host
//! ABI: the host calls `alloc(len)`, writes the input JSON there, calls
//! `execute(ptr, len)`, and reads a little-endian `u32` length followed by the
//! output JSON at the returned pointer.
//!
//! The program never sees another contract. Each operation names the SENDs it
//! takes in, and the RECVs it emits restate them: sender, asset, amount, memo.
//! The sequencer refuses the whole commit when a stated SEND differs from the
//! recorded one, so the numbers here are the numbers that moved.
//!
//! See `docs/tutorials/constant-product-pool.md` for the rules this program
//! runs under.

use serde_json::{json, Map, Value};

pub const LP: &str = "lp";

const TOKEN_A: &str = "/config/token_a.text";
const TOKEN_B: &str = "/config/token_b.text";
const FEE: &str = "/config/fee.num";
const RESERVE_A: &str = "/reserves/a.num";
const RESERVE_B: &str = "/reserves/b.num";
const SUPPLY: &str = "/lp/supply.num";

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

/// A SEND to the pool, as the caller states it.
#[derive(Clone, Debug)]
struct Claim {
    fields: Map<String, Value>,
    from: String,
    asset: String,
    amount: u64,
}

impl Claim {
    fn parse(value: &Value) -> Result<Claim, String> {
        let obj = value.as_object().ok_or("each send must be an object")?;
        let text = |name: &str| obj.get(name).and_then(Value::as_str).filter(|s| !s.is_empty());
        let commit = text("send_commit_id").ok_or("a send needs send_commit_id")?;
        let from = text("from_contract").ok_or("a send needs from_contract")?;
        let id = text("asset_id").ok_or("a send needs asset_id")?;
        let amount = obj
            .get("amount")
            .and_then(Value::as_u64)
            .filter(|a| *a > 0)
            .ok_or("a send needs a positive whole amount")?;
        let index = match obj.get("send_index") {
            None | Some(Value::Null) => 0,
            Some(v) => v.as_u64().ok_or("send_index must be a whole number")?,
        };
        let creator = text("asset_contract");
        let mut fields = Map::new();
        fields.insert("send_commit_id".into(), json!(commit));
        fields.insert("send_index".into(), json!(index));
        fields.insert("from_contract".into(), json!(from));
        fields.insert("asset_id".into(), json!(id));
        fields.insert("amount".into(), json!(amount));
        if let Some(creator) = creator {
            fields.insert("asset_contract".into(), json!(creator));
        }
        if let Some(memo) = obj.get("memo").filter(|m| !m.is_null()) {
            fields.insert("memo".into(), memo.clone());
        }
        Ok(Claim {
            fields,
            from: from.to_string(),
            asset: match creator {
                Some(creator) => format!("{creator}:{id}"),
                None => id.to_string(),
            },
            amount,
        })
    }

    /// The memo's `op`, which must be the operation the SEND is spent on.
    fn memo(&self, op: &str) -> Result<&Map<String, Value>, String> {
        let memo = self
            .fields
            .get("memo")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{op} needs the SEND's memo, with op \"{op}\""))?;
        if memo.get("op").and_then(Value::as_str) != Some(op) {
            return Err(format!("the SEND's memo is not for {op}"));
        }
        Ok(memo)
    }

    fn recv(&self) -> Value {
        json!({"method": "recv", "path": null, "value": Value::Object(self.fields.clone())})
    }
}

struct Pool {
    token_a: String,
    token_b: String,
    fee: (u128, u128),
    a: u64,
    b: u64,
    supply: u64,
}

impl Pool {
    fn read(state: &Map<String, Value>) -> Result<Pool, String> {
        let text = |path: &str| {
            state
                .get(path)
                .and_then(Value::as_str)
                .filter(|s| s.contains(':'))
                .map(str::to_string)
                .ok_or_else(|| format!("{path} must name a held asset as creator:asset_id"))
        };
        let token_a = text(TOKEN_A)?;
        let token_b = text(TOKEN_B)?;
        if token_a == token_b {
            return Err("the pool's two assets must differ".into());
        }
        let fee = state.get(FEE).ok_or("the pool has no fee")?;
        let fee = decimal(&fee.to_string().replace('"', ""))
            .filter(|(n, d)| n < d)
            .ok_or("the fee must be a decimal in [0, 1)")?;
        Ok(Pool {
            token_a,
            token_b,
            fee,
            a: whole(state, RESERVE_A)?,
            b: whole(state, RESERVE_B)?,
            supply: whole(state, SUPPLY)?,
        })
    }

    fn side(&self, asset: &str) -> Option<bool> {
        if asset == self.token_a {
            Some(true)
        } else if asset == self.token_b {
            Some(false)
        } else {
            None
        }
    }

    fn token(&self, is_a: bool) -> &str {
        if is_a {
            &self.token_a
        } else {
            &self.token_b
        }
    }

    fn posts(&self, supply_changed: bool) -> Vec<Value> {
        let mut out = vec![post(RESERVE_A, self.a), post(RESERVE_B, self.b)];
        if supply_changed {
            out.push(post(SUPPLY, self.supply));
        }
        out
    }
}

fn run(input: &Value) -> Result<Vec<Value>, String> {
    let args = input.get("args").ok_or("input has no args")?;
    let state = input
        .pointer("/context/state")
        .and_then(Value::as_object)
        .ok_or("input has no context.state")?;
    let op = args.get("op").and_then(Value::as_str).ok_or("args need an op")?;
    let claims = args
        .get("sends")
        .and_then(Value::as_array)
        .ok_or("args need sends: the SENDs this operation takes in")?
        .iter()
        .map(Claim::parse)
        .collect::<Result<Vec<_>, _>>()?;
    let mut ids: Vec<(&Value, &Value)> =
        claims.iter().map(|c| (&c.fields["send_commit_id"], &c.fields["send_index"])).collect();
    ids.sort_by_key(|(c, i)| (c.to_string(), i.to_string()));
    ids.dedup();
    if ids.len() != claims.len() {
        return Err("a SEND is named twice".into());
    }
    if op == "refund" {
        return refund(&claims);
    }
    let pool = Pool::read(state)?;
    match op {
        "add" => add(pool, &claims),
        "swap" => swap(pool, &claims),
        "remove" => remove(pool, &claims),
        other => Err(format!("unknown op {other:?}; use add, swap, remove, or refund")),
    }
}

/// Take in any SENDs and return each to its sender, unchanged.
fn refund(claims: &[Claim]) -> Result<Vec<Value>, String> {
    if claims.is_empty() {
        return Err("refund needs at least one SEND".into());
    }
    let mut out: Vec<Value> = claims.iter().map(Claim::recv).collect();
    out.extend(claims.iter().map(|c| send(&c.asset, &c.from, c.amount)));
    Ok(out)
}

fn add(mut pool: Pool, claims: &[Claim]) -> Result<Vec<Value>, String> {
    let from = &claims.first().ok_or("add needs the SENDs it deposits")?.from;
    let (mut da, mut db, mut min_shares) = (0u64, 0u64, 0u64);
    for claim in claims {
        let memo = claim.memo("add")?;
        match memo.get("min_shares") {
            None => {}
            Some(v) => min_shares = min_shares.max(v.as_u64().ok_or("min_shares must be a whole number")?),
        }
        if &claim.from != from {
            return Err("an add takes SENDs from one sender".into());
        }
        match pool.side(&claim.asset) {
            Some(true) if da == 0 => da = claim.amount,
            Some(false) if db == 0 => db = claim.amount,
            Some(_) => return Err("an add takes at most one SEND of each asset".into()),
            None => return Err(format!("{} is not one of the pool's assets", claim.asset)),
        }
    }
    // The supply is zero exactly when both reserves are: the first deposit
    // needs both assets, and a full remove pays out everything.
    let shares = if pool.supply == 0 {
        if da == 0 || db == 0 {
            return refund(claims);
        }
        isqrt(da as u128 * db as u128)
    } else {
        let by = |d: u64, r: u64| d as u128 * pool.supply as u128 / r.max(1) as u128;
        by(da, pool.a).min(by(db, pool.b))
    };
    let shares = u64::try_from(shares).map_err(|_| "too many shares")?;
    if shares == 0 || shares < min_shares {
        return refund(claims);
    }
    pool.a = pool.a.checked_add(da).ok_or("reserve overflow")?;
    pool.b = pool.b.checked_add(db).ok_or("reserve overflow")?;
    pool.supply = pool.supply.checked_add(shares).ok_or("supply overflow")?;
    let mut out: Vec<Value> = claims.iter().map(Claim::recv).collect();
    out.push(send(LP, from, shares));
    out.extend(pool.posts(true));
    Ok(out)
}

fn swap(mut pool: Pool, claims: &[Claim]) -> Result<Vec<Value>, String> {
    let [claim] = claims else {
        return Err("a swap takes exactly one SEND".into());
    };
    let memo = claim.memo("swap")?;
    let min_out = memo
        .get("min_out")
        .and_then(Value::as_u64)
        .ok_or("the swap memo needs a whole min_out")?;
    let is_a = pool
        .side(&claim.asset)
        .ok_or_else(|| format!("{} is not one of the pool's assets", claim.asset))?;
    let (r_in, r_out) = if is_a { (pool.a, pool.b) } else { (pool.b, pool.a) };
    if r_in == 0 || r_out == 0 {
        return refund(claims);
    }
    let out = amount_out(claim.amount, r_in, r_out, pool.fee).ok_or("amounts too large for the pool's arithmetic")?;
    if out == 0 || out < min_out {
        return refund(claims);
    }
    let r_in = r_in.checked_add(claim.amount).ok_or("reserve overflow")?;
    let r_out = r_out - out;
    if is_a {
        (pool.a, pool.b) = (r_in, r_out);
    } else {
        (pool.b, pool.a) = (r_in, r_out);
    }
    let mut actions = vec![claim.recv(), send(pool.token(!is_a), &claim.from, out)];
    actions.extend(pool.posts(false));
    Ok(actions)
}

fn remove(mut pool: Pool, claims: &[Claim]) -> Result<Vec<Value>, String> {
    let [claim] = claims else {
        return Err("a remove takes exactly one SEND of shares".into());
    };
    claim.memo("remove")?;
    if claim.asset != LP {
        return Err("a remove takes the pool's own lp shares".into());
    }
    if claim.amount > pool.supply {
        return Err("more shares than the pool issued".into());
    }
    let share = |r: u64| (r as u128 * claim.amount as u128 / pool.supply as u128) as u64;
    let (da, db) = (share(pool.a), share(pool.b));
    pool.a -= da;
    pool.b -= db;
    pool.supply -= claim.amount;
    let mut out = vec![claim.recv()];
    for (is_a, amount) in [(true, da), (false, db)] {
        if amount > 0 {
            out.push(send(pool.token(is_a), &claim.from, amount));
        }
    }
    out.extend(pool.posts(true));
    Ok(out)
}

/// `floor(r_out * x / (r_in + x))` with `x = amount_in * (1 - fee)`, the
/// most the pool can pay and keep `(r_in + x) * (r_out - out) >= r_in * r_out`.
/// `None` when the product does not fit in 128 bits.
pub fn amount_out(amount_in: u64, r_in: u64, r_out: u64, (n, d): (u128, u128)) -> Option<u64> {
    let x = (amount_in as u128).checked_mul(d - n)?;
    let num = (r_out as u128).checked_mul(x)?;
    let den = (r_in as u128).checked_mul(d)?.checked_add(x)?;
    Some((num / den) as u64)
}

pub fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = 1u128 << ((128 - n.leading_zeros()).div_ceil(2));
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// `"0.003"` as `(3, 1000)`; whole numbers too. At most 18 decimal places.
pub fn decimal(s: &str) -> Option<(u128, u128)> {
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if whole.is_empty() || frac.len() > 18 || !(whole.chars().chain(frac.chars()).all(|c| c.is_ascii_digit())) {
        return None;
    }
    let den = 10u128.pow(frac.len() as u32);
    let num = whole.parse::<u128>().ok()?.checked_mul(den)? + if frac.is_empty() { 0 } else { frac.parse::<u128>().ok()? };
    Some((num, den))
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

fn send(asset: &str, to: &str, amount: u64) -> Value {
    let mut value = json!({"to_contract": to, "amount": amount});
    match asset.split_once(':') {
        Some((creator, id)) => {
            value["asset_contract"] = json!(creator);
            value["asset_id"] = json!(id);
        }
        None => value["asset_id"] = json!(asset),
    }
    json!({"method": "send", "path": null, "value": value})
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "KA:tokA";
    const B: &str = "KB:tokB";

    fn state(a: u64, b: u64, supply: u64) -> Value {
        json!({
            TOKEN_A: A, TOKEN_B: B, FEE: 0.003,
            RESERVE_A: a, RESERVE_B: b, SUPPLY: supply,
            "/__programs__/pool.wasm": "AGFzbQ==",
        })
    }

    fn claim(from: &str, asset: &str, amount: u64, memo: Value) -> Value {
        let mut c = json!({"send_commit_id": format!("c-{from}-{asset}"), "from_contract": from, "amount": amount, "memo": memo});
        match asset.split_once(':') {
            Some((creator, id)) => {
                c["asset_contract"] = json!(creator);
                c["asset_id"] = json!(id);
            }
            None => c["asset_id"] = json!(asset),
        }
        c
    }

    fn call(op: &str, sends: Vec<Value>, st: Value) -> Value {
        let input = json!({"args": {"op": op, "sends": sends}, "context": {"state": st}});
        respond(input.to_string().as_bytes())
    }

    fn actions(out: &Value) -> Vec<(String, Value)> {
        assert_eq!(out["errors"], json!([]), "{out}");
        out["actions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| (a["method"].as_str().unwrap().to_string(), a.clone()))
            .collect()
    }

    fn posted(out: &Value, path: &str) -> Option<u64> {
        actions(out).into_iter().find(|(m, a)| m == "post" && a["path"] == path).map(|(_, a)| a["value"].as_u64().unwrap())
    }

    fn sent(out: &Value) -> Vec<(String, String, u64)> {
        actions(out)
            .into_iter()
            .filter(|(m, _)| m == "send")
            .map(|(_, a)| {
                let v = &a["value"];
                let asset = match v.get("asset_contract") {
                    Some(c) => format!("{}:{}", c.as_str().unwrap(), v["asset_id"].as_str().unwrap()),
                    None => v["asset_id"].as_str().unwrap().to_string(),
                };
                (asset, v["to_contract"].as_str().unwrap().to_string(), v["amount"].as_u64().unwrap())
            })
            .collect()
    }

    #[test]
    fn the_first_deposit_mints_the_geometric_mean() {
        let out = call("add", vec![claim("L", A, 1000, json!({"op": "add"})), claim("L", B, 4000, json!({"op": "add"}))], state(0, 0, 0));
        assert_eq!(sent(&out), vec![(LP.into(), "L".into(), 2000)]);
        assert_eq!(posted(&out, RESERVE_A), Some(1000));
        assert_eq!(posted(&out, RESERVE_B), Some(4000));
        assert_eq!(posted(&out, SUPPLY), Some(2000));
        let recvs: Vec<_> = actions(&out).into_iter().filter(|(m, _)| m == "recv").collect();
        assert_eq!(recvs.len(), 2);
        assert_eq!(recvs[0].1["value"]["memo"], json!({"op": "add"}));
    }

    #[test]
    fn a_later_deposit_mints_the_smaller_proportion() {
        let out = call("add", vec![claim("M", A, 100, json!({"op": "add"})), claim("M", B, 1000, json!({"op": "add"}))], state(1000, 4000, 2000));
        assert_eq!(sent(&out), vec![(LP.into(), "M".into(), 200)]);
        assert_eq!(posted(&out, SUPPLY), Some(2200));
    }

    #[test]
    fn a_deposit_below_min_shares_is_returned() {
        let add = |min: u64| json!({"op": "add", "min_shares": min});
        let out = call("add", vec![claim("M", A, 100, add(201)), claim("M", B, 1000, add(0))], state(1000, 4000, 2000));
        assert_eq!(sent(&out), vec![(A.into(), "M".into(), 100), (B.into(), "M".into(), 1000)]);
        assert_eq!(posted(&out, SUPPLY), None);
        let out = call("add", vec![claim("M", A, 100, add(200)), claim("M", B, 1000, add(0))], state(1000, 4000, 2000));
        assert_eq!(sent(&out), vec![(LP.into(), "M".into(), 200)]);
        let one_sided = call("add", vec![claim("L", A, 100, json!({"op": "add"}))], state(0, 0, 0));
        assert_eq!(sent(&one_sided), vec![(A.into(), "L".into(), 100)]);
        let dust = call("add", vec![claim("M", A, 1, json!({"op": "add"})), claim("M", B, 1, json!({"op": "add"}))], state(1000, 4000, 2000));
        assert_eq!(sent(&dust), vec![(A.into(), "M".into(), 1), (B.into(), "M".into(), 1)]);
    }

    #[test]
    fn a_swap_pays_the_curve_less_the_fee() {
        let out = call("swap", vec![claim("T", A, 100, json!({"op": "swap", "min_out": 360}))], state(1000, 4000, 2000));
        assert_eq!(sent(&out), vec![(B.into(), "T".into(), 362)]);
        assert_eq!(posted(&out, RESERVE_A), Some(1100));
        assert_eq!(posted(&out, RESERVE_B), Some(3638));
        assert_eq!(posted(&out, SUPPLY), None);
        let back = call("swap", vec![claim("T", B, 400, json!({"op": "swap", "min_out": 0}))], state(1100, 3638, 2000));
        assert_eq!(sent(&back), vec![(A.into(), "T".into(), 108)]);
    }

    #[test]
    fn a_swap_below_min_out_is_returned() {
        let out = call("swap", vec![claim("T", A, 100, json!({"op": "swap", "min_out": 363}))], state(1000, 4000, 2000));
        assert_eq!(sent(&out), vec![(A.into(), "T".into(), 100)]);
        assert_eq!(posted(&out, RESERVE_A), None);
    }

    #[test]
    fn a_remove_pays_pro_rata_rounded_down() {
        let out = call("remove", vec![claim("L", LP, 1000, json!({"op": "remove"}))], state(1100, 3638, 2000));
        assert_eq!(sent(&out), vec![(A.into(), "L".into(), 550), (B.into(), "L".into(), 1819)]);
        assert_eq!(posted(&out, SUPPLY), Some(1000));
        let all = call("remove", vec![claim("L", LP, 2000, json!({"op": "remove"}))], state(1100, 3638, 2000));
        assert_eq!(posted(&all, RESERVE_A), Some(0));
        assert_eq!(posted(&all, SUPPLY), Some(0));
    }

    #[test]
    fn a_send_is_spent_only_on_what_its_memo_says() {
        let st = || state(1000, 4000, 2000);
        let err = |out: Value| out["errors"][0].as_str().unwrap_or_default().to_string();
        assert!(err(call("swap", vec![claim("T", A, 100, json!({"op": "add"}))], st())).contains("not for swap"));
        assert!(err(call("swap", vec![claim("T", A, 100, Value::Null)], st())).contains("needs the SEND's memo"));
        assert!(err(call("swap", vec![claim("T", A, 100, json!({"op": "swap"}))], st())).contains("min_out"));
        assert!(err(call("remove", vec![claim("L", A, 10, json!({"op": "remove"}))], st())).contains("lp shares"));
        assert!(err(call("add", vec![claim("L", A, 10, json!({"op": "add"})), claim("M", B, 40, json!({"op": "add"}))], st())).contains("one sender"));
        assert!(err(call("swap", vec![claim("T", "KC:tokC", 100, json!({"op": "swap", "min_out": 0}))], st())).contains("not one of"));
        let twice = claim("T", A, 100, json!({"op": "refund"}));
        assert!(err(call("refund", vec![twice.clone(), twice], st())).contains("named twice"));
        let odd = call("refund", vec![claim("T", "KC:tokC", 5, Value::Null)], st());
        assert_eq!(sent(&odd), vec![("KC:tokC".into(), "T".into(), 5)]);
    }

    #[test]
    fn arithmetic() {
        assert_eq!(decimal("0.003"), Some((3, 1000)));
        assert_eq!(decimal("0"), Some((0, 1)));
        assert_eq!(decimal("1e-3"), None);
        assert_eq!(isqrt(4_000_000), 2000);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(u64::MAX as u128 * u64::MAX as u128), u64::MAX as u128);
        assert_eq!(amount_out(u64::MAX, u64::MAX, u64::MAX, (3, 1000)), None);
        for (x, ra, rb) in [(1u64, 1u64, 1u64), (100, 1000, 4000), (1 << 40, 1 << 41, 3 << 39), (7, 1 << 50, 5)] {
            let out = amount_out(x, ra, rb, (3, 1000)).unwrap() as u128;
            let counted = ra as u128 * 1000 + x as u128 * 997;
            assert!(counted * (rb as u128 - out) >= ra as u128 * 1000 * rb as u128);
        }
    }
}
