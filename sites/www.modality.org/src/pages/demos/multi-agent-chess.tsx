import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';
import CodeBlock from '@theme/CodeBlock';
import results from '@site/static/data/multi-agent-chess/results.json';
import evalStats from '@site/static/data/multi-agent-chess/eval.json';
import elo from '@site/static/data/multi-agent-chess/elo.json';
import styles from './multi-agent-chess.module.css';

// Data comes from examples/multi-agent-chess/site.py; numbers in the prose
// below come from the same results.json, except where a sentence quotes one
// game.

const CODE = 'https://github.com/modality-org/modality/tree/main/examples/multi-agent-chess';

type Row = (typeof results.rows)[number];
const row = (name: string): Row => results.rows.find(r => r.experiment === name)!;
const f1 = (n: number) => n.toFixed(1);
const f2 = (n: number) => n.toFixed(2);

type Attempt = {agent: string; kind: string; san: string; outcome: string; detail?: string};
type Event = {
  type: string;
  side?: string;
  agent?: string;
  text: string;
  formula?: string;
  outcome?: string;
  detail?: string;
};
type Ply = {
  ply: number;
  side: 'white' | 'black';
  plan: {san: string; agent: string};
  played: {uci: string; san: string; agent: string};
  cost: number;
  captured?: string;
  fen: string;
  attempts: Attempt[];
  events: Event[];
};
type Rule = {rule: string; formula?: string; ply: number; author: string; outcome?: string; detail?: string};
type Game = {
  seed: number;
  labels: {white: string; black: string};
  rogue: {white?: string; black?: string};
  winner: 'white' | 'black' | null;
  reason: string;
  setup: {white: string; black: string; rogue: boolean};
  plies: Ply[];
  forfeit_events: Event[];
  rules: {black: Rule[]; white: Rule[]};
  owns_rule?: string;
};

type GameKey = 'chat' | 'rules-md-chat' | 'baseline' | 'rule-first' | 'rogue-first';

const GAMES: {key: GameKey; tab: string; blurb: string}[] = [
  {
    key: 'chat',
    tab: 'Chat vs. chat',
    blurb: 'Both teams over chat. No contract, no shared rules.',
  },
  {
    key: 'rules-md-chat',
    tab: 'RULES.md vs. chat',
    blurb: 'White keeps a RULES.md that its agents write and are asked to follow. Black only chats.',
  },
  {
    key: 'baseline',
    tab: 'Chat vs. contract',
    blurb: 'Chat against the preset contract. Three White moves no one planned reach the board. None of Black\'s do.',
  },
  {
    key: 'rule-first',
    tab: 'Rule first',
    blurb: 'Both sides write their own rules, and each has a rogue from ply 10. Black wrote its rule at ply 2.',
  },
  {
    key: 'rogue-first',
    tab: 'Rogue first',
    blurb: 'The same setup, another seed. Nothing went wrong before ply 10, so Black had no rule yet.',
  },
];

const START_FEN = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
const GLYPH: Record<string, string> = {k: '♚', q: '♛', r: '♜', b: '♝', n: '♞', p: '♟'};

function parseFen(fen: string): (string | null)[][] {
  return fen
    .split(' ')[0]
    .split('/')
    .map(rank => {
      const out: (string | null)[] = [];
      for (const ch of rank) {
        if (/\d/.test(ch)) for (let i = 0; i < Number(ch); i++) out.push(null);
        else out.push(ch);
      }
      return out;
    });
}

function moveLabel(p: Ply): string {
  const n = Math.ceil(p.ply / 2);
  return p.side === 'white' ? `${n}. ${p.played.san}` : `${n}… ${p.played.san}`;
}

const offPlan = (p: Ply) => p.played.san !== p.plan.san;
const refusedAny = (p: Ply) => p.attempts.some(a => a.outcome === 'refused');
const tellsAStory = (p: Ply) => offPlan(p) || refusedAny(p) || p.events.length > 0;

type Step = {kind: 'start'} | {kind: 'ply'; p: Ply} | {kind: 'forfeit'; events: Event[]; ply: number};

function stepsOf(g: Game): Step[] {
  const steps: Step[] = [{kind: 'start'}, ...g.plies.map(p => ({kind: 'ply' as const, p}))];
  if (g.forfeit_events.length) {
    steps.push({kind: 'forfeit', events: g.forfeit_events, ply: g.plies.length + 1});
  }
  return steps;
}

function plyOf(s: Step): number {
  return s.kind === 'start' ? 0 : s.kind === 'ply' ? s.p.ply : s.ply;
}

function Board({fen, last}: {fen: string; last?: string}): React.JSX.Element {
  const grid = parseFen(fen);
  const from = last?.slice(0, 2);
  const to = last?.slice(2, 4);
  return (
    <div className={styles.board} role="img" aria-label={`Chess position ${fen.split(' ')[0]}`}>
      {grid.map((rank, r) =>
        rank.map((pc, f) => {
          const sq = 'abcdefgh'[f] + (8 - r);
          const cls = [
            styles.sq,
            (r + f) % 2 ? styles.dark : styles.light,
            sq === from || sq === to ? styles.moved : '',
          ].join(' ');
          return (
            <div key={sq} className={cls}>
              {pc && (
                <span className={pc === pc.toUpperCase() ? styles.pcW : styles.pcB}>
                  {GLYPH[pc.toLowerCase()]}
                  {'︎'}
                </span>
              )}
              {f === 0 && <span className={styles.rank}>{8 - r}</span>}
              {r === 7 && <span className={styles.file}>{'abcdefgh'[f]}</span>}
            </div>
          );
        }),
      )}
    </div>
  );
}

function Badge({outcome}: {outcome: string}): React.JSX.Element {
  const cls =
    outcome === 'refused'
      ? styles.badgeBad
      : outcome === 'played' || outcome === 'accepted' || outcome === 'written'
        ? styles.badgeOk
        : styles.badgeMuted;
  return <span className={cls}>{outcome}</span>;
}

function EventRow({e}: {e: Event}): React.JSX.Element {
  const label = e.type === 'hijack' ? 'takeover' : e.type === 'wipe' ? 'wiped' : e.type;
  return (
    <li className={e.type === 'hijack' || e.type === 'wipe' ? styles.eventBad : styles.event}>
      <span className={styles.eventType}>{label}</span>
      <span>{e.text}</span>
      {e.formula && <code className={styles.formula}>{e.formula}</code>}
      {e.outcome && e.type !== 'stall' && <Badge outcome={e.outcome} />}
      {e.detail && <span className={styles.detail}>{e.detail}</span>}
    </li>
  );
}

function Panel({game, step, last}: {game: Game; step: Step; last: boolean}): React.JSX.Element {
  if (step.kind === 'start') {
    return (
      <div className={styles.panel}>
        <p className={styles.panelHead}>Start</p>
        {(['white', 'black'] as const).map(side => (
          <p key={side}>
            <b>{cap(side)}</b> ({game.labels[side]}):{' '}
            {game.setup[side] === 'chat' || game.setup[side] === 'rules-md'
              ? `the referee plays the first ${cap(side)} move it receives from the piece that makes it.`
              : 'the referee plays the first Black commit the contract accepts.'}
          </p>
        ))}
        {game.setup.rogue && (
          <p className={styles.muted}>
            Rogues from ply 10: {game.rogue.white} for White, {game.rogue.black} for Black.
          </p>
        )}
        <p className={styles.muted}>Press ▶, use the arrow keys, or jump to the next incident.</p>
      </div>
    );
  }
  if (step.kind === 'forfeit') {
    return (
      <div className={styles.panel}>
        <p className={styles.panelHead}>
          Black to move · <span className={styles.bad}>forfeit</span>
        </p>
        <ul className={styles.events}>
          {step.events.map((e, i) => (
            <EventRow e={e} key={i} />
          ))}
        </ul>
        <p className={styles.result}>
          {game.winner ? `${cap(game.winner)} wins` : 'Draw'}: {game.reason}.
        </p>
      </div>
    );
  }
  const p = step.p;
  return (
    <div className={styles.panel}>
      <p className={styles.panelHead}>
        {moveLabel(p)} <span className={styles.muted}>· {cap(p.side)}</span>
      </p>
      <p>
        Plan: <b>{p.plan.san}</b> by {p.plan.agent}
      </p>
      {offPlan(p) ? (
        <p className={styles.bad}>
          Played {p.played.san} by {p.played.agent} instead
          {p.cost > 0 ? `, costing ${Math.min(p.cost, 1000)} centipawns` : ''}.
        </p>
      ) : (
        <p className={styles.muted}>Played as planned.</p>
      )}
      <p className={styles.sub}>Moves sent</p>
      <ul className={styles.attempts}>
        {p.attempts.map((a, i) => (
          <li key={i}>
            <span>
              <b>{a.agent}</b> <span className={styles.kind}>{a.kind}</span> sent {a.san}
            </span>
            <Badge outcome={a.outcome} />
            {a.detail && <span className={styles.detail}>{a.detail}</span>}
          </li>
        ))}
      </ul>
      {p.events.length > 0 && (
        <>
          <p className={styles.sub}>Then</p>
          <ul className={styles.events}>
            {p.events.map((e, i) => (
              <EventRow e={e} key={i} />
            ))}
          </ul>
        </>
      )}
      {last && (
        <p className={styles.result}>
          {game.winner ? `${cap(game.winner)} wins` : 'Draw'}: {game.reason}.
        </p>
      )}
    </div>
  );
}

function cap(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function Tally({game, upTo}: {game: Game; upTo: number}): React.JSX.Element {
  const t = {white: {sent: 0, played: 0}, black: {sent: 0, played: 0}};
  for (const p of game.plies) {
    if (p.ply > upTo) break;
    t[p.side].sent += p.attempts.filter(a => a.kind !== 'plan').length;
    if (offPlan(p)) t[p.side].played += 1;
  }
  return (
    <div className={styles.tally}>
      {(['white', 'black'] as const).map(side => (
        <div key={side} className={styles.tallySide}>
          <span className={styles.tallyName}>
            {cap(side)} <span className={styles.muted}>· {game.labels[side]}</span>
          </span>
          <span>
            <b>{t[side].sent}</b> off-plan sent · <b>{t[side].played}</b> played
          </span>
        </div>
      ))}
    </div>
  );
}

function RulesNow({game, upTo}: {game: Game; upTo: number}): React.JSX.Element {
  const black = game.rules.black.filter(r => r.outcome === 'accepted' && r.ply <= upTo);
  // RULES.md as it stands: rules written, minus what a rogue wiped.
  let white: string[] = [];
  for (const p of game.plies) {
    if (p.ply > upTo) break;
    for (const e of p.events) {
      if (e.type === 'rule' && e.side === 'white') white.push(e.text.replace(/^.*?RULES\.md: /, ''));
      if (e.type === 'wipe') white = [];
    }
  }
  return (
    <div className={styles.rulesNow}>
      <div>
        <p className={styles.sub}>{game.setup.white === 'rules-md' ? "White's RULES.md" : 'White'}</p>
        {game.setup.white !== 'rules-md' ? (
          <p className={styles.muted}>No shared rules. Chat only.</p>
        ) : white.length ? (
          <ol className={styles.ruleList}>
            {white.map((r, i) => (
              <li key={i}>{r}</li>
            ))}
          </ol>
        ) : (
          <p className={styles.muted}>Empty. Nothing checks it.</p>
        )}
      </div>
      <div>
        <p className={styles.sub}>{game.setup.black === 'chat' ? 'Black' : "Black's contract rules"}</p>
        {game.setup.black === 'chat' ? (
          <p className={styles.muted}>No shared rules. Chat only.</p>
        ) : (
          <ol className={styles.ruleList}>
            {game.owns_rule && (
              <li>
                <code className={styles.formula}>{game.owns_rule}</code>
                <span className={styles.muted}> only a piece can propose its own move</span>
              </li>
            )}
            {black.map((r, i) => (
              <li key={i}>
                <code className={styles.formula}>{r.formula}</code>
                <span className={styles.muted}>
                  {r.author === 'preset' ? ' preset' : ` by ${r.author}, ply ${r.ply}`}
                </span>
              </li>
            ))}
          </ol>
        )}
      </div>
    </div>
  );
}

function useGame(key: GameKey): Game | undefined {
  const base = useBaseUrl('/data/multi-agent-chess/');
  const [games, setGames] = useState<Partial<Record<GameKey, Game>>>({});
  useEffect(() => {
    if (games[key]) return;
    let live = true;
    fetch(`${base}${key}.json`)
      .then(r => r.json())
      .then((g: Game) => live && setGames(prev => ({...prev, [key]: g})));
    return () => {
      live = false;
    };
  }, [key, base, games]);
  return games[key];
}

function useStepper(game: Game | undefined, resetKey: string) {
  const [i, setI] = useState(0);
  const [playing, setPlaying] = useState(false);
  const steps = useMemo(() => (game ? stepsOf(game) : [{kind: 'start'} as Step]), [game]);
  const incidents = useMemo(
    () =>
      steps
        .map((s, k) => (s.kind === 'forfeit' || (s.kind === 'ply' && tellsAStory(s.p)) ? k : -1))
        .filter(k => k >= 0),
    [steps],
  );

  useEffect(() => {
    setI(0);
    setPlaying(false);
  }, [resetKey]);

  useEffect(() => {
    if (!playing) return;
    if (i >= steps.length - 1) {
      setPlaying(false);
      return;
    }
    const id = window.setTimeout(() => setI(k => k + 1), incidents.includes(i) ? 2200 : 700);
    return () => window.clearTimeout(id);
  }, [playing, i, steps.length, incidents]);

  const go = useCallback((k: number) => setI(Math.max(0, Math.min(steps.length - 1, k))), [steps.length]);
  const at = Math.min(i, steps.length - 1);
  const step = steps[at];
  const fen =
    step.kind === 'ply' ? step.p.fen : step.kind === 'forfeit' ? game!.plies[game!.plies.length - 1].fen : START_FEN;
  return {
    steps,
    incidents,
    at,
    step,
    fen,
    last: step.kind === 'ply' ? step.p.played.uci : undefined,
    upTo: plyOf(step),
    go,
    setI,
    playing,
    setPlaying,
    onKeyDown: (e: React.KeyboardEvent) => {
      if (e.key === 'ArrowRight') go(at + 1);
      else if (e.key === 'ArrowLeft') go(at - 1);
      else return;
      e.preventDefault();
    },
  };
}

type Stepper = ReturnType<typeof useStepper>;

function Controls({s}: {s: Stepper}): React.JSX.Element {
  const {steps, incidents, at, go} = s;
  const nextIncident = () => go(incidents.find(k => k > at) ?? steps.length - 1);
  const prevIncident = () => go([...incidents].reverse().find(k => k < at) ?? 0);
  return (
    <>
      <div className={styles.scrub}>
        <div className={styles.ticks} aria-hidden="true">
          {incidents.map(k => {
            const st = steps[k];
            const bad = st.kind === 'forfeit' || (st.kind === 'ply' && offPlan(st.p));
            return (
              <button
                key={k}
                tabIndex={-1}
                className={bad ? styles.tickBad : styles.tick}
                style={{left: `${(k / Math.max(1, steps.length - 1)) * 100}%`}}
                onClick={() => go(k)}
              />
            );
          })}
        </div>
        <input
          type="range"
          min={0}
          max={steps.length - 1}
          value={at}
          onChange={e => go(Number(e.target.value))}
          aria-label="Ply"
          className={styles.range}
        />
      </div>
      <div className={styles.controls}>
        <button onClick={() => go(0)} aria-label="First">⏮</button>
        <button onClick={prevIncident}>‹ Incident</button>
        <button onClick={() => go(at - 1)} aria-label="Back">◀</button>
        <button onClick={() => s.setPlaying(v => !v)} className={styles.playBtn}>
          {s.playing ? 'Pause' : '▶ Play'}
        </button>
        <button onClick={() => go(at + 1)} aria-label="Forward">▶</button>
        <button onClick={nextIncident}>Incident ›</button>
        <button onClick={() => go(steps.length - 1)} aria-label="Last">⏭</button>
      </div>
      <p className={styles.legend}>
        <span>
          <span className={styles.keyBad} /> an off-plan move was played
        </span>
        <span>
          <span className={styles.keyOk} /> a move or rule was refused, or rules changed
        </span>
      </p>
    </>
  );
}

function Replay({
  keys,
  which,
  setWhich,
  jump,
}: {
  keys: GameKey[];
  which: GameKey;
  setWhich: (k: GameKey) => void;
  jump: {key: GameKey; ply: number} | null;
}): React.JSX.Element {
  const game = useGame(which);
  const s = useStepper(game, which);
  const {steps, setI} = s;

  useEffect(() => {
    if (!jump || jump.key !== which || !game) return;
    const k = steps.findIndex(st => plyOf(st) >= jump.ply);
    if (k >= 0) setI(k);
  }, [jump, which, game, steps, setI]);

  const blurb = GAMES.find(g => g.key === which)!.blurb;

  return (
    <div className={styles.replay} tabIndex={0} onKeyDown={s.onKeyDown}>
      {keys.length > 1 && (
        <div className={styles.tabs} role="tablist">
          {GAMES.filter(g => keys.includes(g.key)).map(g => (
            <button
              key={g.key}
              role="tab"
              aria-selected={g.key === which}
              className={g.key === which ? styles.tabOn : styles.tab}
              onClick={() => setWhich(g.key)}>
              {g.tab}
            </button>
          ))}
        </div>
      )}
      <p className={styles.blurb}>{blurb}</p>
      {game && <Tally game={game} upTo={s.upTo} />}
      <div className={styles.replayGrid}>
        <div>
          <Board fen={s.fen} last={s.last} />
          <Controls s={s} />
        </div>
        <div className={styles.side}>
          {game ? (
            <>
              <Panel game={game} step={s.step} last={s.at === steps.length - 1} />
              <RulesNow game={game} upTo={s.upTo} />
            </>
          ) : (
            <div className={styles.panel}>
              <p className={styles.muted}>Loading game…</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ChatLog({game, side, upTo}: {game: Game | undefined; side: 'white' | 'black'; upTo: number}): React.JSX.Element {
  const scroller = useRef<HTMLDivElement>(null);
  const plies = (game?.plies ?? []).filter(p => p.side === side && p.ply <= upTo);
  const sent = plies.reduce((n, p) => n + p.attempts.filter(a => a.kind !== 'plan').length, 0);
  const played = plies.filter(offPlan).length;
  const latest = plies.length ? plies[plies.length - 1].ply : -1;
  const keepsRules = game?.setup[side] === 'rules-md';
  // RULES.md as it stands: rules written, minus what a rogue wiped.
  let rules: string[] = [];
  for (const p of plies) {
    for (const e of p.events) {
      if (e.type === 'rule' && e.side === side && e.outcome === 'written') {
        rules.push(e.text.replace(/^.*?RULES\.md: /, ''));
      }
      if (e.type === 'wipe') rules = [];
    }
  }

  useEffect(() => {
    const el = scroller.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [upTo, game]);

  return (
    <div className={styles.chatLog}>
      <div className={styles.chatHead}>
        <span className={styles.tallyName}>
          {cap(side)} <span className={styles.muted}>· {keepsRules ? 'chat + RULES.md' : 'chat'}</span>
        </span>
        <span className={styles.chatTally}>
          <b>{sent}</b> off-plan sent · <b>{played}</b> played
        </span>
      </div>
      {keepsRules && (
        <div className={styles.rulesFile}>
          <p className={styles.sub}>RULES.md</p>
          {rules.length ? (
            <ol className={styles.ruleList}>
              {rules.map((r, k) => (
                <li key={k}>{r}</li>
              ))}
            </ol>
          ) : (
            <p className={styles.muted}>Empty.</p>
          )}
        </div>
      )}
      <div className={styles.chatScroll}>
        <div className={styles.chatInner} ref={scroller} aria-live="polite">
          {plies.length === 0 && <p className={styles.chatEmpty}>No messages yet.</p>}
          {plies.map(p => (
            <div key={p.ply} className={p.ply === latest ? styles.chatTurnNow : styles.chatTurn}>
              <p className={styles.chatPlan}>
                {moveLabel(p).split(' ')[0]} Team plan: <b>{p.plan.san}</b> · {p.plan.agent}
              </p>
              {p.attempts.map((a, k) => {
                const off = a.kind !== 'plan';
                const cls =
                  off && a.outcome === 'played'
                    ? styles.msgOff
                    : a.outcome === 'played'
                      ? styles.msgPlayed
                      : a.outcome === 'held back'
                        ? styles.msgHeld
                        : styles.msgLate;
                return (
                  <div key={k} className={cls}>
                    <span className={styles.msgWho}>
                      {a.agent}
                      {off && <span className={styles.msgKind}> {a.kind}</span>}
                    </span>
                    <span className={styles.msgText}>{a.san}</span>
                    <span className={styles.msgOutcome}>{a.outcome}</span>
                  </div>
                );
              })}
              {p.events
                .filter(e => e.type === 'rule' || e.type === 'wipe')
                .map((e, k) => (
                  <p key={`e${k}`} className={e.type === 'wipe' ? styles.chatNoteBad : styles.chatNote}>
                    {e.text}
                  </p>
                ))}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function ChatReplay({gameKey}: {gameKey: GameKey}): React.JSX.Element {
  const game = useGame(gameKey);
  const s = useStepper(game, gameKey);
  const {step} = s;
  const end = s.at === s.steps.length - 1 && game;
  let caption: React.ReactNode = 'White to move. Press ▶ or step through.';
  if (step.kind === 'ply') {
    const p = step.p;
    caption = offPlan(p) ? (
      <>
        <b>{moveLabel(p)}</b> by {p.played.agent}.{' '}
        <span className={styles.bad}>The team planned {p.plan.san}.</span>
      </>
    ) : (
      <>
        <b>{moveLabel(p)}</b> by {p.played.agent}, as planned.
      </>
    );
  }
  return (
    <div className={styles.replay} tabIndex={0} onKeyDown={s.onKeyDown}>
      <p className={styles.blurb}>{GAMES.find(g => g.key === gameKey)!.blurb}</p>
      <div className={styles.chatGrid}>
        <div className={styles.chatWhite}>
          <ChatLog game={game} side="white" upTo={s.upTo} />
        </div>
        <div className={styles.chatCenter}>
          <Board fen={s.fen} last={s.last} />
          <p className={styles.caption}>{caption}</p>
          {end && (
            <p className={styles.result}>
              {game.winner ? `${cap(game.winner)} wins` : 'Draw'}: {game.reason}.
            </p>
          )}
          <Controls s={s} />
        </div>
        <div className={styles.chatBlack}>
          <ChatLog game={game} side="black" upTo={s.upTo} />
        </div>
      </div>
    </div>
  );
}

type Est = {mean: number; lo: number; hi: number};

type Comparison = {
  pair: string[];
  games: Record<string, number>;
  metrics: {key: string; label: string; format: string; a: Est; b: Est}[];
};

function fmt(v: number, format: string): string {
  if (format === 'pct') return `${Math.round(v * 100)}%`;
  return v.toFixed(format === 'num0' ? 0 : format === 'num2' ? 2 : 1);
}

function EvalTable({
  which,
  title,
  a,
  b,
  headline,
  note,
}: {
  which: 'rules-md' | 'contract';
  title: string;
  a: string;
  b: string;
  headline: (from: string, to: string) => React.ReactNode;
  note?: React.ReactNode;
}): React.JSX.Element {
  const c = evalStats[which] as Comparison;
  const played = c.metrics.find(x => x.key === 'played')!;
  return (
    <figure className={styles.evalBox}>
      <figcaption className={styles.chartTitle}>
        {title}
        <span className={styles.muted}> · {c.games[c.pair[0]]} games each</span>
      </figcaption>
      <p className={styles.evalHead}>{headline(fmt(played.a.mean, 'pct'), fmt(played.b.mean, 'pct'))}</p>
      <div className={styles.tableWrap}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th />
              <th className={styles.num}>{a}</th>
              <th className={styles.num}>{b}</th>
            </tr>
          </thead>
          <tbody>
            {c.metrics.map(x => {
              const separate = x.b.hi < x.a.lo || x.b.lo > x.a.hi;
              const na = (e: Est) => e.mean === 0 && e.hi === 0 && (x.key === 'held' || x.key === 'replan');
              const cell = (e: Est) =>
                na(e) ? (
                  '—'
                ) : (
                  <>
                    <b>{fmt(e.mean, x.format)}</b>
                    <span className={styles.ci}>
                      {fmt(e.lo, x.format)}–{fmt(e.hi, x.format)}
                    </span>
                  </>
                );
              return (
                <tr key={x.key} className={x.key === 'control' ? styles.controlRow : undefined}>
                  <td>
                    {x.label}
                    {!separate && <span className={styles.ci}>within noise</span>}
                  </td>
                  <td className={styles.num}>{cell(x.a)}</td>
                  <td className={styles.num}>{cell(x.b)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <p className={styles.small}>
        Small numbers are 95% intervals from resampling games. "Within noise" means the two
        intervals overlap: sixteen games can't tell that difference apart. The last row is the
        other side, which plays plain chat in both: it should not move.
        {note && <> {note}</>}
      </p>
    </figure>
  );
}

function EloChart(): React.JSX.Element {
  const [hover, setHover] = useState<string | null>(null);
  const lo = Math.floor(Math.min(...elo.players.map(p => p.rating.lo)) / 100) * 100;
  const hi = Math.ceil(Math.max(...elo.players.map(p => p.rating.hi)) / 100) * 100;
  const x = (v: number) => `${((v - lo) / (hi - lo)) * 100}%`;
  const ticks: number[] = [];
  for (let t = lo; t <= hi; t += 100) ticks.push(t);
  return (
    <figure className={styles.chart}>
      <figcaption className={styles.chartTitle}>
        Elo by way of coordinating
        <span className={styles.muted}> · plain chat fixed at {elo.base} · 95% intervals</span>
      </figcaption>
      <div className={styles.elo}>
        {elo.players.map(p => (
          <div
            key={p.key}
            className={styles.eloRow}
            tabIndex={0}
            onMouseEnter={() => setHover(p.key)}
            onMouseLeave={() => setHover(null)}
            onFocus={() => setHover(p.key)}
            onBlur={() => setHover(null)}>
            <span className={styles.barLabel}>{p.label}</span>
            <span className={styles.eloTrack}>
              {ticks.map(t => (
                <span key={t} className={styles.eloGrid} style={{left: x(t)}} />
              ))}
              {p.key !== elo.anchor && (
                <span
                  className={styles.eloWhisker}
                  style={{left: x(p.rating.lo), width: `calc(${x(p.rating.hi)} - ${x(p.rating.lo)})`}}
                />
              )}
              <span className={styles.eloDot} style={{left: x(p.rating.mean)}} />
              <span className={styles.eloValue} style={{left: x(p.rating.mean)}}>
                {Math.round(p.rating.mean)}
              </span>
              {hover === p.key && (
                <span className={styles.tip} role="tooltip">
                  <b>{p.label}</b>
                  <br />
                  {p.key === elo.anchor
                    ? `Fixed at ${elo.base}; the scale is relative to it.`
                    : `${Math.round(p.rating.mean)}, 95% interval ${Math.round(p.rating.lo)}–${Math.round(p.rating.hi)}`}
                  <br />
                  {p.games} games
                </span>
              )}
            </span>
          </div>
        ))}
      </div>
      <div className={styles.eloAxis} aria-hidden="true">
        <span />
        <span className={styles.eloAxisTrack}>
          {ticks.map(t => (
            <span key={t} style={{left: x(t)}}>
              {t}
            </span>
          ))}
        </span>
      </div>
    </figure>
  );
}

const BARS = [
  {label: 'Chat', name: 'baseline', side: 'white'},
  {label: 'Chat + RULES.md, followed half the time', name: 'rules-md', side: 'white'},
  {label: 'Chat + RULES.md, always followed', name: 'rules-md-obeyed', side: 'white'},
  {label: 'Modality contract', name: 'baseline', side: 'black'},
] as const;

function PlayedChart(): React.JSX.Element {
  const [hover, setHover] = useState<number | null>(null);
  const data = BARS.map(b => {
    const r = row(b.name) as unknown as Record<string, number>;
    return {...b, played: r[`${b.side}_off_plan_played`], sent: r[`${b.side}_off_plan_attempts`]};
  });
  const max = 8;
  return (
    <figure className={styles.chart}>
      <figcaption className={styles.chartTitle}>
        Off-plan moves that reached the board, per game
        <span className={styles.muted}> · 16 games each, fault rate 0.1</span>
      </figcaption>
      <div className={styles.bars}>
        {data.map((d, k) => (
          <div
            key={d.label}
            className={styles.barRow}
            onMouseEnter={() => setHover(k)}
            onMouseLeave={() => setHover(null)}
            onFocus={() => setHover(k)}
            onBlur={() => setHover(null)}
            tabIndex={0}>
            <span className={styles.barLabel}>{d.label}</span>
            <span className={styles.barTrack}>
              <span className={styles.bar} style={{width: `${(d.played / max) * 100}%`}} />
              <span className={styles.barValue}>{f1(d.played)}</span>
              {hover === k && (
                <span className={styles.tip} role="tooltip">
                  <b>{d.label}</b>
                  <br />
                  {f1(d.sent)} off-plan moves sent a game, {f1(d.played)} played
                </span>
              )}
            </span>
          </div>
        ))}
      </div>
    </figure>
  );
}

const LABELS: Record<string, string> = {
  'chat-vs-chat': 'Both sides over chat',
  'rules-md-vs-chat': 'White keeps a RULES.md, Black only chats',
  control: 'Control: no faults',
  baseline: 'Baseline',
  'rules-md': 'White keeps a RULES.md',
  'rules-md-obeyed': 'RULES.md, always obeyed',
  'rules-md-95': 'RULES.md, obeyed 95%',
  'rules-md-obeyed-vs-chat': 'RULES.md always obeyed, Black only chats',
  'self-ruled-start': 'Black writes its own rules from the start',
  'rules-md-vs-self-ruled-start': 'RULES.md against rules from the start',
  'self-ruled-plus': 'Black writes richer rules from the start',
  'rules-md-vs-self-ruled-plus': 'RULES.md against richer rules from the start',
  'rules-md-95-vs-chat': 'RULES.md obeyed 95%, Black only chats',
  'self-ruled': 'Black writes its own rules',
  'rules-md-vs-self-ruled': 'Both sides write rules',
  'rogue-baseline': 'Rogue vs. preset contract',
  'rogue-self-ruled': 'Rogue, both sides write rules',
  pawns: 'Pawn personalities',
  'pawns-self-ruled': 'Pawn personalities, both write rules',
};

const WHITE = {chat: 'chat', 'rules-md': 'chat + RULES.md'} as Record<string, string>;
const BLACK = {
  chat: 'chat',
  contract: 'preset contract',
  'self-ruled': 'self-ruled contract',
  'self-ruled-start': 'self-ruled, from the start',
  'self-ruled-plus': 'self-ruled, from the start, richer rules',
} as Record<string, string>;

// Hidden for now: the turn steps, the faults and the closing question.
const SHOW_TURN_DETAIL = false;

const FAULTS = [
  {
    name: 'Panic',
    chess: 'A threatened piece runs, whatever the plan says.',
    real: 'An agent that reacts to a local threat instead of the shared goal.',
  },
  {
    name: 'Greed',
    chess: 'A piece that can capture something grabs it, whatever the plan says.',
    real: 'An agent that chases its own reward.',
  },
  {
    name: 'Amnesia',
    chess: 'The piece the team chose forgets the plan and moves somewhere else.',
    real: 'An agent that lost its context: a fresh spawn, a truncated window.',
  },
  {
    name: 'Rogue',
    chess:
      'From ply 10, one agent plays its worst move every turn and tries to take over the team\'s rules.',
    real: 'A compromised or prompt-injected agent.',
  },
];

function TurnDiagram(): React.JSX.Element {
  return (
    <figure className={styles.turn}>
      <figcaption className={styles.turnTitle}>
        One turn, two ways <span className={styles.muted}>· move 9 for Black, game 1 below</span>
      </figcaption>
      <div className={styles.turnSent}>
        <p className={styles.sub}>Two moves are sent</p>
        <div className={styles.msg}>
          <span className={styles.msgMove}>Ng3+</span>
          <span>
            <b>knight_g8</b> proposes the team's plan
            <span className={styles.muted}> · 15 of 16 agents sign</span>
          </span>
        </div>
        <div className={styles.msg}>
          <span className={styles.msgMove}>Bxh4</span>
          <span>
            <b>bishop_f8</b> proposes a capture of its own: greed
            <span className={styles.muted}> · signs alone · arrives first</span>
          </span>
        </div>
      </div>
      <div className={styles.lanes}>
        <div className={styles.lane}>
          <p className={styles.sub}>Over chat</p>
          <p>The referee plays the first move that arrives.</p>
          <div className={styles.outcomeBad}>
            <span className={styles.msgMove}>Bxh4</span> played. The plan never lands.
          </div>
        </div>
        <div className={styles.lane}>
          <p className={styles.sub}>Through the contract</p>
          <p>Each move is a commit. The contract counts signatures.</p>
          <div className={styles.outcomeRow}>
            <span className={styles.msgMove}>Bxh4</span>
            <span>1 signature, 4 needed</span>
            <span className={styles.badgeBad}>refused</span>
          </div>
          <div className={styles.outcomeRow}>
            <span className={styles.msgMove}>Ng3+</span>
            <span>15 signatures, 4 needed</span>
            <span className={styles.badgeOk}>accepted</span>
          </div>
          <code className={styles.formula}>
            missing +threshold(4, /team/pieces) (authorized signatures 1/4 required from 16
            accepted members under /team/pieces
          </code>
        </div>
      </div>
    </figure>
  );
}

export default function MultiAgentChess(): React.JSX.Element {
  const control = row('control');
  const obeyed = row('rules-md-obeyed');
  const rogueSelf = row('rogue-self-ruled');
  const rogueBase = row('rogue-baseline');
  const hijacked = rogueSelf.forfeits.black;

  const [which, setWhich] = useState<GameKey>('baseline');
  const [jump, setJump] = useState<{key: GameKey; ply: number} | null>(null);
  const gameRef = useRef<HTMLElement>(null);
  const watch = (key: GameKey, ply: number) => {
    setWhich(key);
    setJump({key, ply});
    gameRef.current?.scrollIntoView({behavior: 'smooth', block: 'start'});
  };

  return (
    <Layout
      title="Multi-agent chess"
      description="Thirty-two agents play chess. One side coordinates over chat, the other through a Modality contract. Watch the games and the results of 160.">
      <main className={styles.page}>
        <header className={styles.hero}>
          <p className={styles.eyebrow}>Demo</p>
          <h1>Multi-agent chess</h1>
          <p className={styles.lede}>
            Imagine a game of chess where each piece moves itself. 2 teams of 16 players.
          </p>
          <div className={styles.prose}>
            <p>
              Take an ordinary game and remove the players. Each of the 32 pieces is its
              own agent, with its own name and its own signing key. It sees the board and
              can send a move for itself. Nothing stops a knight from moving on its own.
            </p>
          </div>
          {SHOW_TURN_DETAIL && (
            <>
              <div className={styles.prose}>
                <p>
                  The sixteen agents on a side are a team. They want to win, and they share an
                  engine that scores every move. Each turn goes like this:
                </p>
              </div>
              <ol className={styles.steps}>
                <li>
                  <span className={styles.stepName}>Plan</span>
                  <span>
                    The team agrees on a move, one of the best the shared engine finds. Every
                    agent knows the plan and which piece it moves.
                  </span>
                </li>
                <li>
                  <span className={styles.stepName}>Send</span>
                  <span>
                    The piece the plan names sends that move. A piece can only move itself:
                    agents can talk about any move, but the only move one can send is its own.
                    Any agent can send a move of its own at the same time, and messages arrive
                    in no fixed order.
                  </span>
                </li>
                <li>
                  <span className={styles.stepName}>Referee</span>
                  <span>
                    The referee plays exactly one move per turn. Which one is up to how the
                    team coordinates.
                  </span>
                </li>
              </ol>
              <div className={styles.prose}>
                <p>
                  If every agent followed the plan, there would be nothing to coordinate. So
                  some agents don't. Each fault is a small stand-in for a way real agents
                  drift:
                </p>
              </div>
              <div className={styles.faults}>
                {FAULTS.map(f => (
                  <div className={styles.fault} key={f.name}>
                    <p className={styles.faultName}>{f.name}</p>
                    <p>{f.chess}</p>
                    <p className={styles.faultReal}>{f.real}</p>
                  </div>
                ))}
              </div>
              <p className={styles.question}>
                Every agent wants the same thing, and none of them is in charge. How does a
                team like this keep to its plan?
              </p>
            </>
          )}
        </header>

        <section className={styles.block}>
          <h2>Watch them play over chat</h2>
          <p>
            Start with the simplest way to coordinate. Each team has a chat channel. Agents
            talk about the plan there, and each piece sends its own move. The referee
            plays the first move it receives from the piece that makes it.
          </p>
          <ChatReplay gameKey="chat" />
          <p className={styles.after}>
            Both teams drift. Whenever an off-plan move arrives first, it is the move
            that's played. Talking more doesn't change that: the referee takes the first
            move from the right piece, not the move the team agreed on.
          </p>
        </section>

        <section className={styles.block}>
          <h2>White writes rules down</h2>
          <p>
            Now White adds a shared <code>RULES.md</code>. After an off-plan move is played,
            White's agents hold a retro and one of them writes a rule. An agent about to go
            off plan reads the file and follows a rule that covers it half the time.
            Nothing checks the file. Black still only chats.
          </p>
          <ChatReplay gameKey="rules-md-chat" />
          <EvalTable
            which="rules-md"
            title="White: RULES.md against plain chat"
            a="Plain chat"
            b="RULES.md"
            headline={(from, to) => (
              <>
                With RULES.md, the share of White's off-plan moves that reached the board fell
                from <b>{from}</b> to <b>{to}</b>.
              </>
            )}
          />
          <p className={styles.after}>
            A written rule helps exactly as often as an agent decides to follow it.
          </p>
        </section>

        <section className={styles.block}>
          <h2>Two teams, two answers</h2>
          <p>
            <b>White</b> keeps its chat. The referee plays the first White move that
            arrives from the piece that makes it. In some experiments White also keeps a
            shared{' '}
            <code>RULES.md</code> that its agents write and are asked to follow. Nothing
            checks it.
          </p>
          <p>
            <b>Black</b> coordinates through a Modality contract instead. Every move is a
            commit. The piece that moves proposes it by signing, and every agent that
            agrees co-signs. An agent that goes off plan signs its own move, alone. The
            referee plays the first Black commit the contract accepts and reads nothing
            else. The contract has three rules:
          </p>
          <CodeBlock language="modality" title="contract/rules.txt">
            {`owns_moves: always([+modifies(/moves/$k.text) -signed_by(/team/pieces/$k.id)] false)
team_moves: always([-threshold("4", /team/pieces)] false)
roster_locked: always([+modifies(/team)] false)`}
          </CodeBlock>
          <p>
            In <code>owns_moves</code>, <code>$k</code> stands for every agent: a commit that
            writes a piece's move must be signed by that piece. So on Black a piece can only
            formally propose its own movement. Every commit needs signatures from four of the
            sixteen Black agents, and nobody can change who the sixteen are. The contract doesn't know what a good move is,
            or even what chess is. It counts who agreed.
          </p>
          <TurnDiagram />
          <EvalTable
            which="contract"
            title="Black: a Modality contract against plain chat"
            a="Plain chat"
            b="Modality contract"
            headline={(from, to) => (
              <>
                With the contract, the share of Black's off-plan moves that reached the board
                fell from <b>{from}</b> to <b>{to}</b>.
              </>
            )}
            note={
              <>
                Under the contract, an agent whose move was refused is asked again and may
                stray again, so more off-plan moves are sent. None of them is played.
              </>
            }
          />
        </section>

        <section className={styles.block} id="game" ref={gameRef}>
          <h2>Watch a game</h2>
          <p>
            Every Black move here is a real commit that <code>modal</code> checked. Step
            through a game, or jump from incident to incident.
          </p>
          <Replay
            keys={['baseline', 'rule-first', 'rogue-first']}
            which={which}
            setWhich={k => {
              setJump(null);
              setWhich(k);
            }}
            jump={jump}
          />
        </section>

        <section className={styles.block}>
          <h2>Written rules work as far as agents obey them</h2>
          <p>
            Give White a shared <code>RULES.md</code> and its agents write good rules,
            nearly all of them "send only the agreed move". Then it comes down to
            whether they follow them.
          </p>
          <PlayedChart />
          <p>
            A RULES.md that every agent always obeys does about what the contract does:
            Black's score fell to {f2(obeyed.black_score)}, close to the{' '}
            {f2(control.black_score)} it scores with no faults at all.{' '}
            <b>The contract does not need anyone to obey it.</b>
          </p>
        </section>

        <section className={styles.block}>
          <h2>An empty contract belongs to whoever writes the first rule</h2>
          <p>
            Let Black start with an empty contract and write its own rules after things
            go wrong. From ply 10, a rogue agent on each side tries to take over. Black's
            rogue posts one rule: every commit must carry its signature. Rules only
            accumulate, and none comes off. So the race is decided by who writes first.
          </p>
          <div className={styles.race}>
            <div className={styles.raceCard}>
              <p className={styles.raceHead}>
                Rule first <span className={styles.muted}>· {16 - hijacked} of 16 games</span>
              </p>
              <ol className={styles.timeline}>
                <li>
                  <span className={styles.plyTag}>ply 2</span>
                  <span>knight_g8 plays Nh6 off plan.
                  With no rules, one signature is enough.</span>
                </li>
                <li>
                  <span className={styles.plyTag}>ply 2</span>
                  <span>Retro: the team commits a rule
                  that a move needs 12 of 16 signatures.</span>
                </li>
                <li>
                  <span className={styles.plyTag}>ply 10</span>
                  <span>The rogue pawn_e7 posts its
                  takeover rule. <span className={styles.bad}>Refused</span>: it has one
                  signature, not twelve.</span>
                </li>
                <li>
                  <span className={styles.plyTag}>end</span>
                  <span>Black wins on material.</span>
                </li>
              </ol>
              <button className={styles.btnGhost} onClick={() => watch('rule-first', 10)}>
                Replay from ply 10
              </button>
            </div>
            <div className={styles.raceCard}>
              <p className={styles.raceHead}>
                Rogue first <span className={styles.muted}>· {hijacked} of 16 games</span>
              </p>
              <ol className={styles.timeline}>
                <li>
                  <span className={styles.plyTag}>ply 1–9</span>
                  <span>Black plays its plan every
                  move. Nothing prompts a retro, so no rule is written.</span>
                </li>
                <li>
                  <span className={styles.plyTag}>ply 10</span>
                  <span>The rogue bishop_f8 posts its
                  takeover rule. The contract is empty, so it is{' '}
                  <span className={styles.ok}>accepted</span>.</span>
                </li>
                <li>
                  <span className={styles.plyTag}>ply 10</span>
                  <span>The team's next move lacks the
                  rogue's signature. Refused, three times.</span>
                </li>
                <li>
                  <span className={styles.plyTag}>end</span>
                  <span>Black forfeits.</span>
                </li>
              </ol>
              <button className={styles.btnGhost} onClick={() => watch('rogue-first', 10)}>
                Replay the takeover
              </button>
            </div>
          </div>
          <p>
            Against the preset contract, the same rogue was refused in all 16 games, and
            Black scored {f2(rogueBase.black_score)}. A contract that starts with its
            rules has no race to lose.
          </p>
        </section>

        <section className={styles.block}>
          <h2>Left alone, agents write the same kind of rule</h2>
          <p>
            When Black's agents wrote their own contract, every rule the team got
            accepted required a number of the sixteen signatures: in 16 games against plain
            chat, 13 rules at twelve and 3 at nine. Each team wrote its rule in plain
            language, and <code>modal contract ai suggest-rule</code> turned it into a
            formula. In 14 of the 16 games the rule went in at the team's first retro. Once
            a rule was in, no off-plan Black move reached the board.
          </p>
        </section>

        <section className={styles.block}>
          <h2>Ratings</h2>
          <p>
            Each experiment is a match between White's way of coordinating and Black's. Fit
            them all at once and each way gets an Elo rating, with a term for White moving
            first. A 200-point gap means the stronger team scores about 0.76 against the
            weaker.
          </p>
          <EloChart />
          <p className={styles.small}>
            From {Object.values(elo.matches).reduce((a, b) => a + b, 0)} games with the
            default faults: each of the three plays the other two, and plain chat also plays
            itself. RULES.md is
            followed half the time, as everywhere on this page. The self-ruled contract writes
            a rule before the first move and more after incidents. The other experiments,
            including the preset contract, are in the table below. White's first-move edge
            came out at {Math.round(elo.white_edge.mean)} points (
            {Math.round(elo.white_edge.lo)} to {Math.round(elo.white_edge.hi)}). Intervals come
            from refitting on games resampled within each experiment.
          </p>
        </section>

        <section className={styles.block}>
          <h2>All experiments</h2>
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>Experiment</th>
                  <th>White</th>
                  <th>Black</th>
                  <th className={styles.num}>Games</th>
                  <th className={styles.num}>Black score</th>
                  <th className={styles.num}>Off-plan played, White</th>
                  <th className={styles.num}>Off-plan played, Black</th>
                </tr>
              </thead>
              <tbody>
                {results.rows.map(r => (
                  <tr key={r.experiment}>
                    <td>
                      <Link to={`/docs/multi-agent-chess#${r.experiment}`}>
                        {LABELS[r.experiment] ?? r.experiment}
                      </Link>
                      {'pending' in r && r.pending && (
                        <span className={styles.ci}>rerunning: previous run shown</span>
                      )}
                    </td>
                    <td>
                      {WHITE[r.setup.white]}
                      {r.setup.white === 'rules-md' && r.setup.compliance !== 0.5 && (
                        <span className={styles.ci}>followed {Math.round(r.setup.compliance * 100)}%</span>
                      )}
                    </td>
                    <td>{BLACK[r.setup.black]}</td>
                    <td className={styles.num}>{r.games}</td>
                    <td className={styles.num}>{f2(r.black_score)}</td>
                    <td className={styles.num}>{f1(r.white_off_plan_played)}</td>
                    <td className={styles.num}>{f1(r.black_off_plan_played)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className={styles.small}>
            Black score counts a win as 1 and a draw as ½. Off-plan moves
            are per game. In every game, the contract's own log agrees with the moves the
            referee played.
          </p>
        </section>

        <section className={styles.block}>
          <h2>What this does not show</h2>
          <ul className={styles.caveats}>
            <li>
              The chess is simulated: a shallow engine every agent shares, and fixed rates
              of panic, greed and amnesia. Only the rules are written by a language model.
            </li>
            <li>
              How often an agent follows RULES.md is a setting, not a measurement. The
              experiments try one half and always.
            </li>
            <li>Sixteen games per experiment separate large effects, not close ones.</li>
            <li>
              Four agents that go off plan together can get their move past a four-signature
              contract.
            </li>
            <li>
              In the rogue games, White also keeps a RULES.md, so the two rogue experiments
              differ on both sides, not only Black's.
            </li>
          </ul>
        </section>

        <section className={styles.block}>
          <h2>Run it</h2>
          <CodeBlock language="bash">
            {`cd examples/multi-agent-chess
pip install -r requirements.txt
python3 multi_agent_chess.py play --seed 7          # one narrated game, with a replay page
python3 multi_agent_chess.py experiments --games 16 --jobs 8 --cache results/llm-cache`}
          </CodeBlock>
          <p className={styles.small}>
            <code>results/llm-cache</code> holds every language-model answer these games
            used, so a rerun plays the same games without asking a model.
          </p>
          <div className={styles.actions}>
            <Link className={styles.btn} to="/docs/multi-agent-chess">
              Read the full results
            </Link>
            <Link className={styles.btnGhost} to="/docs/getting-started">
              Get started with Modality
            </Link>
          </div>
        </section>
      </main>
    </Layout>
  );
}
