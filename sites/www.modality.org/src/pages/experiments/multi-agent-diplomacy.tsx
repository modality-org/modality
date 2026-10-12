import React, {useEffect, useMemo, useState} from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';
import CodeBlock from '@theme/CodeBlock';
import results from '@site/static/data/multi-agent-diplomacy/results.json';
import ruleData from '@site/static/data/multi-agent-diplomacy/rules.json';
import base from './multi-agent-chess.module.css';
import styles from './multi-agent-diplomacy.module.css';

// Data comes from examples/multi-agent-diplomacy/site.py: one seed replayed in
// three worlds, and the averages of every experiment run.

const CODE = 'https://github.com/modality-org/modality/tree/main/examples/multi-agent-diplomacy';

type Row = (typeof results.rows)[number];
const row = (name: string): Row => results.rows.find(r => r.experiment === name)!;
const pct = (x: number) => `${Math.round(x * 100)}%`;
const f1 = (x: number) => x.toFixed(1);

type Promise = {kind: string; promiser: string; to: string; provinces: string[]; order: string};
type Broken = {kind: string; by: string; against: string; order: string; promise: string; claim?: string};
type Phase = {
  phase: string;
  promises: Promise[];
  broken: Broken[];
  refused: {promise: string; order: string}[];
  centres: Record<string, number>;
  tokens: Record<string, number>;
};
type World = {
  seed: number;
  phases: Phase[];
  centres: Record<string, number>;
  tokens: Record<string, number>;
  score: Record<string, number>;
  winner: string | null;
};
type WorldKey = 'chat' | 'contract' | 'deposit';

const WORLDS: {key: WorldKey; tab: string; blurb: string}[] = [
  {
    key: 'chat',
    tab: 'Chat',
    blurb: 'Promises are words. A betrayal is played, and the betrayed power never deals with the betrayer again.',
  },
  {
    key: 'contract',
    tab: 'Strict contract',
    blurb: 'Promises are Modality rules both powers sign. A betrayal is refused, and the power plays what it promised.',
  },
  {
    key: 'deposit',
    tab: 'Deposit',
    blurb:
      'Promises are rules with a one-centre deposit in escrow. A betrayal is played, and the victim takes the deposit, citing it.',
  },
];

const POWERS = ['AUSTRIA', 'ENGLAND', 'FRANCE', 'GERMANY', 'ITALY', 'RUSSIA', 'TURKEY'];
const title = (s: string) => s.charAt(0) + s.slice(1).toLowerCase();
const season = (phase: string) => `${phase[0] === 'S' ? 'Spring' : 'Fall'} ${phase.slice(1, 5)}`;

function Chip({power}: {power: string}): React.JSX.Element {
  return <span className={`${styles.chip} ${styles[power.toLowerCase()]}`}>{title(power)}</span>;
}

function promiseText(p: Promise): React.ReactNode {
  if (p.kind === 'pact') {
    return (
      <>
        <Chip power={p.promiser} /> stays out of {p.provinces.join(', ')}
      </>
    );
  }
  return (
    <>
      <Chip power={p.promiser} /> orders <code>{p.order}</code> for <Chip power={p.to} />
    </>
  );
}

function useWorlds(): Record<WorldKey, World> | undefined {
  const url = useBaseUrl('/data/multi-agent-diplomacy/worlds.json');
  const [data, setData] = useState<Record<WorldKey, World>>();
  useEffect(() => {
    let live = true;
    fetch(url)
      .then(r => r.json())
      .then(d => live && setData(d));
    return () => {
      live = false;
    };
  }, [url]);
  return data;
}

function Replay(): React.JSX.Element {
  const worlds = useWorlds();
  const frames = useBaseUrl('/data/multi-agent-diplomacy/frames/');
  const [which, setWhich] = useState<WorldKey>('chat');
  const [i, setI] = useState(0);
  const [playing, setPlaying] = useState(false);
  const world = worlds?.[which];
  const n = world ? world.phases.length : 1;
  const at = Math.min(i, n - 1);
  const ph = world?.phases[at];
  const incidents = useMemo(
    () => (world ? world.phases.map((p, k) => (p.broken.length || p.refused.length ? k : -1)).filter(k => k >= 0) : []),
    [world],
  );
  useEffect(() => {
    if (!playing) return;
    if (at >= n - 1) {
      setPlaying(false);
      return;
    }
    const id = window.setTimeout(() => setI(k => k + 1), incidents.includes(at) ? 2600 : 1400);
    return () => window.clearTimeout(id);
  }, [playing, at, n, incidents]);
  const go = (k: number) => setI(Math.max(0, Math.min(n - 1, k)));
  const last = at === n - 1;
  const totals = (key: 'broken' | 'refused' | 'promises') =>
    world ? world.phases.slice(0, at + 1).reduce((s, p) => s + p[key].length, 0) : 0;

  return (
    <div
      className={base.replay}
      tabIndex={0}
      onKeyDown={e => {
        if (e.key === 'ArrowRight') go(at + 1);
        else if (e.key === 'ArrowLeft') go(at - 1);
        else return;
        e.preventDefault();
      }}>
      <div className={base.tabs} role="tablist">
        {WORLDS.map(w => (
          <button
            key={w.key}
            role="tab"
            aria-selected={w.key === which}
            className={w.key === which ? base.tabOn : base.tab}
            onClick={() => setWhich(w.key)}>
            {w.tab}
          </button>
        ))}
      </div>
      <p className={base.blurb}>{WORLDS.find(w => w.key === which)!.blurb}</p>
      <div className={styles.dipGrid}>
        <div>
          <div className={styles.mapWrap}>
            {ph ? (
              <img
                src={`${frames}${which}/${ph.phase}.svg`}
                alt={`The map in ${season(ph.phase)}, ${which}, with every order drawn`}
                className={styles.map}
              />
            ) : (
              <div className={styles.mapLoading}>Loading the game…</div>
            )}
          </div>
          <div className={base.scrub}>
            <div className={base.ticks} aria-hidden="true">
              {incidents.map(k => (
                <button
                  key={k}
                  tabIndex={-1}
                  className={which === 'contract' ? base.tick : base.tickBad}
                  style={{left: `${(k / Math.max(1, n - 1)) * 100}%`}}
                  onClick={() => go(k)}
                />
              ))}
            </div>
            <input
              type="range"
              min={0}
              max={n - 1}
              value={at}
              onChange={e => go(Number(e.target.value))}
              aria-label="Turn"
              className={base.range}
            />
          </div>
          <div className={base.controls}>
            <button onClick={() => go(0)} aria-label="First">⏮</button>
            <button onClick={() => go(at - 1)} aria-label="Back">◀</button>
            <button onClick={() => setPlaying(v => !v)} className={base.playBtn}>
              {playing ? 'Pause' : '▶ Play'}
            </button>
            <button onClick={() => go(at + 1)} aria-label="Forward">▶</button>
            <button onClick={() => go(incidents.find(k => k > at) ?? n - 1)}>Next betrayal ›</button>
          </div>
          <p className={base.legend}>
            <span>
              <span className={base.keyBad} /> a promise was broken
            </span>
            <span>
              <span className={base.keyOk} /> a betrayal was refused
            </span>
          </p>
        </div>
        <div className={styles.side}>
          {ph && world ? (
            <>
              <div className={base.panel}>
                <p className={base.panelHead}>
                  {season(ph.phase)} <span className={base.muted}>· seed {world.seed}</span>
                </p>
                <p className={styles.running}>
                  So far: <b>{totals('promises')}</b> promises made, <b>{totals('broken')}</b> broken
                  {which === 'contract' && (
                    <>
                      , <b>{totals('refused')}</b> refused
                    </>
                  )}
                </p>
                {ph.broken.map((b, k) => (
                  <div key={`b${k}`} className={styles.eventBad}>
                    <span className={styles.eventTag}>Broken</span>
                    <span>
                      <Chip power={b.by} /> breaks its {b.kind === 'pact' ? 'pact with' : 'support for'}{' '}
                      <Chip power={b.against} />: <code>{b.order}</code>
                    </span>
                    {b.claim && (
                      <span className={styles.claim}>
                        {b.claim === 'accepted' ? (
                          <>
                            <Chip power={b.against} /> takes the deposit, citing the order.
                          </>
                        ) : (
                          'The deposit was already taken.'
                        )}
                      </span>
                    )}
                  </div>
                ))}
                {ph.refused.map((x, k) => (
                  <div key={`r${k}`} className={styles.eventRefused}>
                    <span className={styles.eventTagOk}>Refused</span>
                    <span>
                      <code>{x.order}</code> would break: {x.promise}
                    </span>
                  </div>
                ))}
                {!ph.broken.length && !ph.refused.length && (
                  <p className={base.muted}>Every promise kept this turn.</p>
                )}
                {ph.promises.length > 0 && (
                  <details className={styles.signed}>
                    <summary>
                      {ph.promises.length} promise{ph.promises.length === 1 ? '' : 's'} signed this turn
                    </summary>
                    <ul>
                      {ph.promises.map((p, k) => (
                        <li key={k}>{promiseText(p)}</li>
                      ))}
                    </ul>
                  </details>
                )}
                {last && (
                  <p className={base.result}>
                    After 1906: {world.winner ? `${title(world.winner)} leads` : 'no single leader'}.
                  </p>
                )}
              </div>
              <div className={base.panel}>
                <p className={base.sub}>{last ? 'Final score' : 'Supply centres at the start of the turn'}</p>
                <table className={styles.scores}>
                  <tbody>
                    {POWERS.map(p => {
                      const centres = last ? world.centres[p] : ph.centres[p];
                      const tokens = last ? world.tokens[p] : ph.tokens[p];
                      return (
                        <tr key={p}>
                          <td>
                            <Chip power={p} />
                          </td>
                          <td className={base.num}>{centres}</td>
                          {which === 'deposit' && (
                            <td className={base.num}>
                              {tokens > 0 ? '+' : ''}
                              {tokens ? f1(tokens) : '·'}
                            </td>
                          )}
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
                {which === 'deposit' && (
                  <p className={base.small}>Centres, then deposits won (+) or lost (−).</p>
                )}
              </div>
            </>
          ) : (
            <div className={base.panel}>
              <p className={base.muted}>Loading…</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function WorldCards(): React.JSX.Element {
  const cards: {key: string; name: string; tag: string; main: React.ReactNode; note: React.ReactNode}[] = [
    {
      key: 'chat',
      name: 'Chat',
      tag: 'Promises are words',
      main: pct(row('chat').broken_share),
      note: <>of promises broken · {Math.round(row('chat').promises)} promises a game</>,
    },
    {
      key: 'contract',
      name: 'Strict contract',
      tag: 'Betrayal is refused',
      main: pct(row('contract').broken_share),
      note: (
        <>
          broken · {Math.round(row('contract').promises)} promises a game · {Math.round(row('contract').refused)}{' '}
          betrayals refused
        </>
      ),
    },
    {
      key: 'deposit-1',
      name: 'Deposit',
      tag: 'Betrayal has a price',
      main: pct(row('deposit-1').broken_share),
      note: (
        <>
          broken · {Math.round(row('deposit-1').promises)} promises a game · the victim took the deposit{' '}
          {Math.round(row('deposit-1').claimed)} times a game
        </>
      ),
    },
  ];
  return (
    <div className={styles.worldGrid}>
      {cards.map(c => (
        <div key={c.key} className={base.stat}>
          <span className={base.statSide}>
            {c.name} · {c.tag}
          </span>
          <span className={base.statNum}>{c.main}</span>
          <span className={base.statNote}>{c.note}</span>
        </div>
      ))}
    </div>
  );
}

const DEPOSIT_BARS = [
  {label: 'Chat: no deposit, grudges', name: 'chat'},
  {label: 'Deposit of ½ centre', name: 'deposit-0.5'},
  {label: 'Deposit of 1 centre', name: 'deposit-1'},
  {label: 'Deposit of 1½ centres', name: 'deposit-1.5'},
  {label: 'Strict contract', name: 'contract'},
];

function DepositChart(): React.JSX.Element {
  const [hover, setHover] = useState<number | null>(null);
  return (
    <figure className={base.chart}>
      <figcaption className={base.chartTitle}>
        Share of promises broken
        <span className={base.muted}> · 16 games each, temptation 0.3</span>
      </figcaption>
      <div className={base.bars}>
        {DEPOSIT_BARS.map((d, k) => {
          const r = row(d.name);
          return (
            <div
              key={d.name}
              className={base.barRow}
              tabIndex={0}
              onMouseEnter={() => setHover(k)}
              onMouseLeave={() => setHover(null)}
              onFocus={() => setHover(k)}
              onBlur={() => setHover(null)}>
              <span className={base.barLabel}>{d.label}</span>
              <span className={base.barTrack}>
                <span className={base.bar} style={{width: `${r.broken_share * 100}%`}} />
                <span className={base.barValue}>
                  {pct(r.broken_share)}
                  <span className={base.muted}> · {Math.round(r.promises)} promises a game</span>
                </span>
                {hover === k && (
                  <span className={base.tip} role="tooltip">
                    <b>{d.label}</b>
                    <br />
                    {f1(r.promises)} promises a game, {f1(r.broken)} broken
                    {r.refused ? `, ${f1(r.refused)} refused` : ''}
                    {r.claimed ? `, ${f1(r.claimed)} deposits taken` : ''}
                  </span>
                )}
              </span>
            </div>
          );
        })}
      </div>
    </figure>
  );
}

type ShownRule = {key: string; words: string; formula: string; display?: string; tokens?: unknown};

const RULE_TITLES: Record<string, string> = {
  pact: 'A pact, strict',
  support: 'A support, strict',
  deposit: 'A pact, with a deposit',
};
const RULE_NOTES: Record<string, React.ReactNode> = {
  pact: <>No commit may write an order into these provinces in these turns, so a betraying move is refused.</>,
  support: <>The power's orders for the turn must include the promised support, or they are refused.</>,
  deposit: (
    <>
      Only the victim may touch the escrow. It may take the deposit only once a betraying order is in the contract's
      state as evidence, and the deposit can only ever be held by the promiser or the victim.
    </>
  ),
};

function Rules(): React.JSX.Element {
  return (
    <>
      {(ruleData.rules as ShownRule[]).map(r => (
        <div key={r.key} className={base.ruleCard}>
          <p className={base.ruleMeta}>
            <b>{RULE_TITLES[r.key]}</b>
          </p>
          <p className={base.sub}>In plain English</p>
          <blockquote className={base.ruleWords}>{r.words}</blockquote>
          <p className={base.sub}>In Modality</p>
          <CodeBlock language="modality" {...({'data-shiki': JSON.stringify(r.tokens)} as object)}>
            {r.display ?? r.formula}
          </CodeBlock>
          <p className={base.small}>{RULE_NOTES[r.key]}</p>
        </div>
      ))}
    </>
  );
}

const CHECKS: [string, boolean][] = [
  ['France and Germany sign a pact: France stays out of Munich', true],
  ['France breaks it with A BUR - MUN', false],
  ["Germany writes France's orders", false],
  ['Germany promises to support France into Burgundy', true],
  ['Germany leaves the promised support out of its orders', false],
  ['Germany gives the support', true],
  ['France keeps the pact', true],
  ['France moves the turn on (only the referee may)', false],
  ["They sign a pact with France's deposit in escrow", true],
  ['Germany posts a model that would let it take the deposit without evidence', false],
  ['Germany takes the deposit before any betrayal', false],
  ['France breaks the pact (deposit mode: deception stays possible)', true],
  ['Germany takes the deposit, citing the betrayal', true],
  ['France takes the deposit back alone', false],
  ['Germany hands the deposit to Italy', false],
];

const LABELS: Record<string, string> = {
  chat: 'Chat',
  'honest-chat': 'Chat, nobody tempted',
  'tempted-chat': 'Chat, twice the temptation',
  contract: 'Strict contract',
  'tempted-contract': 'Strict contract, twice the temptation',
  'deposit-0.5': 'Deposit of ½ centre',
  'deposit-1': 'Deposit of 1 centre',
  'deposit-1.5': 'Deposit of 1½ centres',
  'tempted-deposit-1': 'Deposit of 1 centre, twice the temptation',
};

export default function MultiAgentDiplomacy(): React.JSX.Element {
  const chat = row('chat');
  const honest = row('honest-chat');
  const d1 = row('deposit-1');
  const lead = (name: string) => row(name).top_betrayer_lead;
  return (
    <Layout
      title="Multi-agent Diplomacy"
      description="Seven agents play Diplomacy, the board game of promises and betrayal. Over chat, a strict Modality contract, and contracts with deposits.">
      <main className={base.page}>
        <header className={base.hero}>
          <p className={base.eyebrow}>Experiment</p>
          <h1>Multi-agent Diplomacy</h1>
          <p className={base.lede}>
            The board game of promises and betrayal, played by seven agents. What changes when a promise is a
            contract?
          </p>
          <div className={styles.prose}>
            <p>
              Each spring, neighbouring powers sign pacts to stay out of each other's supply centres. Each turn,
              allies promise to support each other's attacks. When breaking a promise would win a power a centre, it
              is tempted, and sometimes it breaks it. We played the same games in three worlds:
            </p>
          </div>
          <WorldCards />
          <div className={base.actions}>
            <a className={base.btn} href="#replay">
              Watch one game, three ways
            </a>
            <Link className={base.btnGhost} href={CODE}>
              Code
            </Link>
          </div>
        </header>

        <section className={base.block} id="replay">
          <h2>One game, three worlds</h2>
          <p>
            The same seed, the same powers, the same temptations. Switch worlds on any turn to see the same betrayal
            played, refused, or paid for. The map is the game engine's own, with every order drawn.
          </p>
          <Replay />
        </section>

        <section className={base.block}>
          <h2>Over chat, trust runs out</h2>
          <p>
            Over chat, {pct(chat.broken_share)} of promises were broken. Each betrayed power then refuses to deal with
            the betrayer, so deals dry up: {Math.round(chat.promises)} promises a game, against{' '}
            {Math.round(honest.promises)} when nobody is tempted at all.
          </p>
        </section>

        <section className={base.block}>
          <h2>A strict contract plays like a table of honest powers</h2>
          <p>
            With promises as contract rules, every betrayal is refused: {f1(row('contract').refused)} a game, and{' '}
            {f1(row('tempted-contract').refused)} at twice the temptation. A refused power plays what it promised, so
            the games go move for move as they do when nobody is tempted, with the same{' '}
            {Math.round(row('contract').promises)} promises a game. Betrayal is not punished. It never happens.
          </p>
        </section>

        <section className={base.block}>
          <h2>A deposit puts a price on betrayal</h2>
          <p>
            Diplomacy without deception is a different game. So in the third world, promises are rules with a
            deposit: breaking one is allowed, and the victim takes the deposit, citing the betraying order. A power
            breaks a promise only when it values the gain above the deposit.
          </p>
          <DepositChart />
          <p>
            With a one-centre deposit, {pct(d1.broken_share)} of promises were broken, and powers kept dealing:{' '}
            {Math.round(d1.promises)} promises a game, because a victim that is paid does not hold a grudge. Betrayal
            stopped paying: the power that broke the most promises ended {Math.abs(lead('deposit-1') ?? 0).toFixed(1)}{' '}
            points behind the others on average, against {Math.abs(lead('chat') ?? 0).toFixed(1)} over chat.
          </p>
        </section>

        <section className={base.block}>
          <h2>What a promise is</h2>
          <p>
            Every power's orders for a turn are a commit to one shared contract, signed with its key. A promise is a
            rule both powers sign. These are rules from the games, as committed.
          </p>
          <Rules />
        </section>

        <section className={base.block}>
          <h2>What the contract guarantees</h2>
          <p>
            Checked by hand against a real <code>modal</code> (<code>check_contract.py</code>):
          </p>
          <ul className={styles.checks}>
            {CHECKS.map(([what, ok]) => (
              <li key={what}>
                <span className={ok ? base.badgeOk : base.badgeBad}>{ok ? 'accepted' : 'refused'}</span> {what}
              </li>
            ))}
          </ul>
          <p className={base.small}>
            After every contract game, the orders the referee played are read back from the contract's log and
            compared. Every game matched.
          </p>
        </section>

        <section className={base.block}>
          <h2>All experiments</h2>
          <div className={base.tableWrap}>
            <table className={base.table}>
              <thead>
                <tr>
                  <th>Experiment</th>
                  <th className={base.num}>Games</th>
                  <th className={base.num}>Promises</th>
                  <th className={base.num}>Broken</th>
                  <th className={base.num}>Refused</th>
                  <th className={base.num}>Deposits taken</th>
                  <th className={base.num}>Share broken</th>
                </tr>
              </thead>
              <tbody>
                {results.rows.map(r => (
                  <tr key={r.experiment}>
                    <td>{LABELS[r.experiment] ?? r.experiment}</td>
                    <td className={base.num}>{r.games}</td>
                    <td className={base.num}>{f1(r.promises)}</td>
                    <td className={base.num}>{f1(r.broken)}</td>
                    <td className={base.num}>{f1(r.refused)}</td>
                    <td className={base.num}>{f1(r.claimed)}</td>
                    <td className={base.num}>{pct(r.broken_share)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className={base.small}>Per game, from 1901 to 1906. Temptation is 0.3 unless the row says otherwise.</p>
        </section>

        <section className={base.block}>
          <h2>What this does not show</h2>
          <ul className={base.caveats}>
            <li>
              No language model plays. The powers are scripted and greedy for supply centres, and the promises and
              betrayals are scripted too. These results are about the mechanism, not about how real negotiators
              behave.
            </li>
            <li>
              A power values a gain at between half a centre and one and a half, by design. So a deposit of one and a
              half centres stops every betrayal by construction: what the experiments show is how betrayal falls as
              the deposit rises, not where real players' threshold lies.
            </li>
            <li>
              A strict contract matches the honest games because a refused power plays what it promised. That is the
              design, not a discovery; the finding is that a contract makes it so without anyone being honest.
            </li>
            <li>
              Games run from 1901 to 1906. A strict contract checks every rule on every commit, and with a pact rule
              for every promise a ten-year game takes over an hour.
            </li>
          </ul>
        </section>

        <section className={base.block}>
          <h2>Run it</h2>
          <CodeBlock language="bash">
            {`cd examples/multi-agent-diplomacy
pip install -r requirements.txt
python3 check_contract.py                                  # what the contract refuses
python3 diplomacy_demo.py play --coordination deposit --seed 9
python3 diplomacy_demo.py experiments --games 16 --jobs 8`}
          </CodeBlock>
          <div className={base.actions}>
            <Link className={base.btn} href={CODE}>
              The code
            </Link>
            <Link className={base.btnGhost} to="/experiments/multi-agent-chess">
              Multi-agent chess
            </Link>
          </div>
        </section>
      </main>
    </Layout>
  );
}
