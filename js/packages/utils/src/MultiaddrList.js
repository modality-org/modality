import dns from "dns";

function removeQuotes(str) {
  const m = str.match(/"(.+)"/);
  if (m) {
    return m[1];
  }
  return str;
}

export function matchesPeerIdSuffix(str, peer_id) {
  const re = new RegExp(`\\/p2p\\/${peer_id}$`);
  return str.match(re);
}

async function resolveViaGoogleDns(name, type) {
  const headers = { accept: "application/dns-json" };
  return fetch(`https://dns.google/resolve?name=${name}&type=${type}`, {
    headers,
  }).then((r) => r.json());
}

async function resolveViaCloudflareDns(name, type) {
  const headers = { accept: "application/dns-json" };
  return fetch(
    `https://cloudflare-dns.com/dns-query?name=${name}&type=${type}`,
    { headers }
  ).then((r) => r.json());
}

async function resolveViaWebBasedDns(name, type) {
  const ans = await resolveViaGoogleDns(name, type);
  return ans.Answer?.map((i) => i.data);
}

async function resolveViaDns(name, type) {
  return dns.promises.resolve(name, type);
}

export async function resolveDnsEntries(entries) {
  const r = [];
  for (const entry of entries) {
    const p2p_match = entry.match(/\/p2p\/(.+)$/);
    if (entry.match(/^\/dns\//)) {
      const m = entry.match(/^\/dns\/([A-Za-z0-9-.]+)(.*)/);
      const name = m[1];
      const rest = m[2];
      const answers = await resolveViaDns(name, "A");
      for (const address of answers) {
        const ans = `/ip4/${address}${rest}`;
        if (!p2p_match || matchesPeerIdSuffix(ans, p2p_match[1])) {
          r.push(ans);
        }
      }
    } else if (entry.match(/^\/dnsaddr\//)) {
      const m = entry.match(/^\/dnsaddr\/([A-Za-z0-9-.]+)(.*)/);
      const name = `_dnsaddr.${m[1]}`;
      let answers = await resolveViaDns(name, "TXT");
      answers = answers.flat().map((ans) => removeQuotes(ans));
      for (const answer of answers) {
        const ans_match = answer.match(/^dnsaddr=(.*)/);
        if (ans_match) {
          const ans = ans_match[1];
          if (!p2p_match || matchesPeerIdSuffix(ans, p2p_match[1])) {
            r.push(ans);
          }
        }
      }
    } else {
      r.push(entry);
    }
  }
  return r;
}
