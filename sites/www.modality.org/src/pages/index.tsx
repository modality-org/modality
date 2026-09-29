import React, {useEffect, useRef, useState} from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import styles from './index.module.css';

const RIVER =
  'a verification language for AI agent cooperation   ·   negotiate and verify   ·   modal contracts   ·   append-only logs of signed commits   ·   prove commitments with temporal logic   ·   ';

function visible(el: Element, root: Element): boolean {
  let cur: Element | null = el;
  while (cur && cur !== root) {
    if (parseFloat(getComputedStyle(cur).opacity) < 0.4) return false;
    cur = cur.parentElement;
  }
  return true;
}

function Witness({
  marker,
  place,
  children,
}: {
  marker: string;
  place: string;
  children: React.ReactNode;
}): JSX.Element {
  const ref = useRef<SVGSVGElement>(null);

  useEffect(() => {
    const svg = ref.current;
    const walker = svg?.querySelector<SVGCircleElement>('[data-walker]');
    if (!svg || !walker) return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

    const root = svg.parentElement ?? svg;
    let frame = 0;
    let stopped = false;
    let edge: SVGPathElement | null = null;
    let hold: SVGCircleElement | null = null;
    let started = 0;
    let dur = 2600;
    let departAt = 0;
    let arm = 0;

    const nodes = () =>
      Array.from(svg.querySelectorAll<SVGCircleElement>('[data-node]'));

    const seed = () =>
      nodes().find(n => n.dataset.seed === '1') ?? nodes()[0] ?? null;

    const mark = (id: string | undefined) => {
      for (const node of nodes()) {
        node.dataset.hot = node.dataset.node === id ? '1' : '0';
      }
    };

    const sit = (node: SVGCircleElement) => {
      walker.setAttribute('cx', String(node.cx.baseVal.value));
      walker.setAttribute('cy', String(node.cy.baseVal.value));
      mark(node.dataset.node);
    };

    const outgoing = (id: string) =>
      Array.from(svg.querySelectorAll<SVGPathElement>('path[data-from]')).filter(
        path => path.dataset.from === id && visible(path, root),
      );

    const tick = (now: number) => {
      if (stopped) return;
      const faded = parseFloat(getComputedStyle(svg).opacity) < 0.08;
      if (faded) {
        edge = null;
        departAt = 0;
        hold = seed();
        walker.dataset.go = '0';
        if (hold) sit(hold);
        frame = requestAnimationFrame(tick);
        return;
      }
      walker.dataset.go = '1';

      if (edge) {
        const t = (now - started) / dur;
        if (t >= 1 || !visible(edge, root)) {
          hold =
            nodes().find(n => n.dataset.node === edge?.dataset.to) ?? seed();
          edge = null;
          departAt = now + 420;
        } else {
          const len = edge.getTotalLength() || 1;
          const pt = edge.getPointAtLength(Math.min(1, Math.max(0, t)) * len);
          walker.setAttribute('cx', String(pt.x));
          walker.setAttribute('cy', String(pt.y));
          mark(t < 0.5 ? edge.dataset.from : edge.dataset.to);
          frame = requestAnimationFrame(tick);
          return;
        }
      }

      const here = hold ?? seed();
      if (!here) {
        frame = requestAnimationFrame(tick);
        return;
      }
      sit(here);
      if (now < departAt) {
        frame = requestAnimationFrame(tick);
        return;
      }
      const choices = outgoing(here.dataset.node ?? '');
      const forward = choices.filter(path => path.dataset.to !== path.dataset.from);
      const pool = forward.length > 0 ? forward : choices;
      if (!pool.length) {
        frame = requestAnimationFrame(tick);
        return;
      }
      const next = pool[arm % pool.length];
      arm += 1;
      edge = next;
      started = now;
      dur = (Number(next.dataset.dur) || 2.6) * 1000;
      hold = null;
      frame = requestAnimationFrame(tick);
    };

    frame = requestAnimationFrame(tick);
    return () => {
      stopped = true;
      cancelAnimationFrame(frame);
    };
  }, []);

  return (
    <svg
      ref={ref}
      className={`${styles.model} ${place}`}
      viewBox="0 0 160 112"
      aria-hidden="true"
    >
      <defs>
        <marker
          id={marker}
          viewBox="0 0 8 8"
          refX="7"
          refY="4"
          markerWidth="5"
          markerHeight="5"
          orient="auto"
        >
          <path
            d="M0 0.8 L7 4 L0 7.2"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </marker>
      </defs>
      {children}
      <circle r="3.3" className={styles.runner} data-walker="" />
    </svg>
  );
}

function Dot({
  id,
  x,
  y,
  hot,
}: {
  id: string;
  x: number;
  y: number;
  hot?: boolean;
}): JSX.Element {
  return (
    <circle
      className={styles.dot}
      data-node={id}
      data-seed={hot ? '1' : undefined}
      cx={x}
      cy={y}
      r="5.5"
    />
  );
}

function Later({
  step,
  children,
}: {
  step: 2 | 3;
  children: React.ReactNode;
}): JSX.Element {
  return <g className={step === 2 ? styles.later : styles.further}>{children}</g>;
}

function Edge({
  d,
  marker,
  from,
  to,
  dur = 2.6,
}: {
  d: string;
  marker: string;
  from: string;
  to: string;
  dur?: number;
}): JSX.Element {
  return (
    <path
      className={styles.edge}
      d={d}
      pathLength={1}
      markerEnd={`url(#${marker})`}
      data-from={from}
      data-to={to}
      data-dur={dur}
    />
  );
}

type Beat = {
  step: string;
  title: string;
  by: string;
  note: string;
  rule?: {say: string; formula: string};
  adds: string[];
};

const BEATS: Beat[] = [
  {
    step: 'Empty',
    title: 'An empty contract',
    by: 'nobody yet',
    note: 'No rules. Any commit is accepted.',
    adds: [],
  },
  {
    step: 'Names',
    title: 'scout claims a name',
    by: 'scout',
    note: 'scout posts its key and the first rule. From the next commit on, only scout writes under /agents/scout.',
    rule: {
      say: "Only an agent's key writes under its name",
      formula: 'always([+modifies(/agents/$k) -signed_by(/agents/$k.id)] false)',
    },
    adds: ['agents/scout.id', 'agents/scout/plan.md'],
  },
  {
    step: 'Members',
    title: 'Only agents commit',
    by: 'builder, reviewer',
    note: 'builder and reviewer post their keys. Then a rule: every later commit carries an agent’s signature. A stranger’s commit is refused.',
    rule: {
      say: 'Every commit is signed by an agent',
      formula: 'always([-any_signed(/agents)] false)',
    },
    adds: ['agents/builder.id', 'agents/reviewer.id'],
  },
  {
    step: 'Admission',
    title: 'Two let a new agent in',
    by: 'builder',
    note: 'One agent could post a second key it holds and count it twice. Now a new key needs two agents on the commit.',
    rule: {
      say: 'A new key needs two agents',
      formula:
        'always([+post_to_path(/agents/$k.id) -state_exists(/agents/$k.id) -threshold("2", /agents)] false)',
    },
    adds: ['agents/builder/src/'],
  },
  {
    step: 'Release',
    title: 'Two to ship',
    by: 'reviewer',
    note: 'Anything under /release needs two agents on the same commit.',
    rule: {
      say: 'The release needs two agents',
      formula: 'always([+modifies(/release) -threshold("2", /agents)] false)',
    },
    adds: ['agents/reviewer/review.md'],
  },
  {
    step: 'Lock',
    title: 'Two to add a rule',
    by: 'scout',
    note: 'The last rule any one agent adds alone. Rules only accumulate, so none of these can be dropped later.',
    rule: {
      say: 'A new rule needs two agents',
      formula: 'always([+modifies(/rules) -threshold("2", /agents)] false)',
    },
    adds: [],
  },
  {
    step: 'Ship',
    title: 'They ship',
    by: 'builder + reviewer',
    note: 'builder alone writes /release: refused, and the log does not grow. builder and reviewer sign the same commit: accepted.',
    adds: ['release/v1.json'],
  },
];

function stateTree(upTo: number): {line: string; fresh: boolean}[] {
  const rows: {line: string; fresh: boolean}[] = [];
  BEATS.slice(0, upTo + 1).forEach((beat, i) => {
    beat.adds.forEach(line => rows.push({line, fresh: i === upTo}));
  });
  return rows;
}

function RuleLog(): JSX.Element {
  const [at, setAt] = useState(0);
  const [auto, setAuto] = useState(true);
  const root = useRef<HTMLDivElement>(null);
  const last = BEATS.length - 1;

  useEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      setAuto(false);
      setAt(last);
      return;
    }
    const el = root.current;
    if (!el) return;
    const seen = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setAt(0);
          seen.disconnect();
        }
      },
      {threshold: 0.35},
    );
    seen.observe(el);
    return () => seen.disconnect();
  }, [last]);

  useEffect(() => {
    if (!auto) return;
    const wait = at === last ? 7000 : 3800;
    const id = window.setTimeout(() => setAt(at === last ? 0 : at + 1), wait);
    return () => window.clearTimeout(id);
  }, [at, auto, last]);

  const beat = BEATS[at];
  const rules = BEATS.slice(0, at + 1).filter(b => b.rule);
  const tree = stateTree(at);

  return (
    <div className={styles.ruleLog} ref={root}>
      <ol className={styles.beats}>
        {BEATS.map((b, i) => (
          <li key={b.step}>
            <button
              type="button"
              className={
                i === at ? styles.beatOn : i < at ? styles.beatDone : styles.beat
              }
              aria-current={i === at ? 'step' : undefined}
              onClick={() => {
                setAuto(false);
                setAt(i);
              }}
            >
              <span className={styles.beatDot} />
              <span className={styles.beatName}>{b.step}</span>
            </button>
          </li>
        ))}
      </ol>
      <div className={styles.beatHead} key={`head-${at}`}>
        <div className={styles.caption}>
          step {at + 1} of {BEATS.length} · signed by {beat.by}
        </div>
        <h3>{beat.title}</h3>
        <p>{beat.note}</p>
      </div>
      <div className={styles.panes}>
        <div className={styles.pane}>
          <div className={styles.caption}>rules/</div>
          <pre className={styles.paneBody}>
            {rules.length === 0 ? (
              <span className={styles.paneEmpty}>// none yet</span>
            ) : (
              rules.map(b => (
                <span
                  key={b.step}
                  className={b === beat ? styles.ruleFresh : styles.ruleOld}
                >
                  {`// ${b.rule?.say}\n${b.rule?.formula}`}
                </span>
              ))
            )}
          </pre>
        </div>
        <div className={styles.pane}>
          <div className={styles.caption}>state/</div>
          <pre className={styles.paneBody}>
            {tree.length === 0 ? (
              <span className={styles.paneEmpty}>// empty</span>
            ) : (
              tree.map(row => (
                <span
                  key={row.line}
                  className={row.fresh ? styles.lineFresh : styles.lineOld}
                >
                  {row.line}
                </span>
              ))
            )}
          </pre>
        </div>
      </div>
      <div
        className={at === last ? styles.verdicts : styles.verdictsHidden}
        aria-hidden={at !== last}
      >
        <div className={styles.verdictBad}>
          <span className={styles.captionRefused}>Refused</span>
          <code>--sign builder --path /release/v1.json</code>
        </div>
        <div className={styles.verdictOk}>
          <span className={styles.captionOk}>Accepted</span>
          <code>--sign builder --sign reviewer --path /release/v1.json</code>
        </div>
      </div>
    </div>
  );
}

const CHECKS = [
  {
    name: 'Replay',
    body: 'Every node replays the signed log from the first commit and lands on the same state of the model. Here, q1.',
  },
  {
    name: 'Match',
    body: 'The next commit must take an edge out of that state whose labels hold on it. builder alone writes /release: the top edge forbids that write, and the bottom edge needs two keys. No edge fits, so the commit is refused. With reviewer’s key on the same commit, the bottom edge fits.',
  },
  {
    name: 'Rules',
    body: 'A rule is a formula over the model, checked when it is added. Every later model must meet it. An agent can post a model with a looser edge, but the rule forbids that edge, so the model is refused. Models can be replaced. Rules cannot.',
  },
];

const PHASE_CHECK = [0, 1, 1, 2];
const PHASE_WAIT = [3600, 5200, 4200, 5800];
const CHECK_PHASE = [0, 1, 3];
const LOG = [
  'scout · names rule',
  'reviewer · signed rule',
  'builder · release rule',
  'scout · lock rule',
];

function Mechanism(): JSX.Element {
  const [phase, setPhase] = useState(1);
  const [auto, setAuto] = useState(true);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      setAuto(false);
      return;
    }
    const el = root.current;
    if (!el) return;
    const seen = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setPhase(0);
          seen.disconnect();
        }
      },
      {threshold: 0.35},
    );
    seen.observe(el);
    return () => seen.disconnect();
  }, []);

  useEffect(() => {
    if (!auto) return;
    const id = window.setTimeout(
      () => setPhase((phase + 1) % PHASE_WAIT.length),
      PHASE_WAIT[phase],
    );
    return () => window.clearTimeout(id);
  }, [phase, auto]);

  const tone = (bad: boolean, good: boolean) =>
    bad ? styles.mBad : good ? styles.mGood : styles.mIdle;
  const arrow = (bad: boolean, good: boolean) =>
    `url(#${bad ? 'mk-bad' : good ? 'mk-good' : 'mk-idle'})`;

  const entryGood = phase === 0;
  const topBad = phase === 1;
  const lowBad = phase === 1;
  const lowGood = phase === 2;
  const ghost = phase === 3;
  const joined = phase >= 2;

  return (
    <div className={styles.mech} ref={root}>
      <svg
        className={`${styles.mechArt} ${phase === 0 ? styles.replaying : ''}`}
        viewBox="0 0 760 300"
        role="img"
        aria-label="A signed log, a model with two edges out of q1, and a rule that forbids writing /release without two agents"
      >
        <defs>
          {(['idle', 'bad', 'good'] as const).map(k => (
            <marker
              key={k}
              id={`mk-${k}`}
              viewBox="0 0 8 8"
              refX="7"
              refY="4"
              markerWidth="6"
              markerHeight="6"
              orient="auto"
            >
              <path
                className={
                  k === 'bad' ? styles.mBad : k === 'good' ? styles.mGood : styles.mIdle
                }
                d="M0 0.8 L7 4 L0 7.2"
                fill="none"
                strokeWidth="1.3"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </marker>
          ))}
        </defs>

        <text className={styles.mCap} x="24" y="24">log</text>
        <text className={styles.mCap} x="252" y="24">model</text>
        <text className={styles.mCap} x="584" y="24">rule</text>

        <line className={styles.mSpine} x1="36" y1="52" x2="36" y2={joined ? 220 : 180} />
        {LOG.map((label, i) => (
          <g key={label}>
            <circle
              className={styles.mLogDot}
              style={{'--i': i} as React.CSSProperties}
              cx="36"
              cy={60 + i * 40}
              r="5"
            />
            <rect className={styles.mCard} x="52" y={48 + i * 40} width="180" height="24" rx="3" />
            <text className={styles.mText} x="60" y={64 + i * 40}>{label}</text>
          </g>
        ))}

        <g className={joined ? styles.mShow : styles.mHide}>
          <circle className={phase === 2 ? styles.mDotGood : styles.mLogDot} cx="36" cy="220" r="5" />
          <rect
            className={phase === 2 ? styles.mCardGood : styles.mCard}
            x="52"
            y="208"
            width="180"
            height="24"
            rx="3"
          />
          <text className={phase === 2 ? styles.mTextGood : styles.mText} x="60" y="224">
            builder + reviewer · v1
          </text>
        </g>

        <g className={phase === 1 ? styles.mShow : styles.mHide}>
          <path className={styles.mCross} d="M30 246 l12 12 M42 246 l-12 12" />
          <rect className={styles.mCardBad} x="60" y="240" width="198" height="24" rx="3" />
          <text className={styles.mTextBad} x="68" y="256">builder · /release/v1.json</text>
        </g>

        <path
          className={tone(false, entryGood)}
          d="M286 150 L 375 150"
          fill="none"
          strokeWidth="1.5"
          markerEnd={arrow(false, entryGood)}
        />
        <path
          className={tone(topBad, false)}
          d="M380 141 C 330 88, 450 88, 401 141"
          fill="none"
          strokeWidth="1.5"
          markerEnd={arrow(topBad, false)}
        />
        <text className={topBad ? styles.mLabelBad : styles.mLabel} x="390" y="66" textAnchor="middle">
          +any_signed(/agents)
        </text>
        <text className={topBad ? styles.mLabelBad : styles.mLabel} x="390" y="82" textAnchor="middle">
          -modifies(/release)
        </text>
        <path
          className={tone(lowBad, lowGood)}
          d="M380 159 C 330 212, 450 212, 401 160"
          fill="none"
          strokeWidth="1.5"
          markerEnd={arrow(lowBad, lowGood)}
        />
        <text
          className={lowBad ? styles.mLabelBad : lowGood ? styles.mLabelGood : styles.mLabel}
          x="390"
          y="228"
          textAnchor="middle"
        >
          +threshold("2", /agents)
        </text>
        <g className={ghost ? styles.mShow : styles.mHide}>
          <path
            className={styles.mBad}
            d="M402 144 C 466 112, 466 188, 403 156"
            fill="none"
            strokeWidth="1.5"
            strokeDasharray="4 3"
            markerEnd="url(#mk-bad)"
          />
          <text className={styles.mLabelBad} x="470" y="146">+any_signed</text>
          <text className={styles.mLabelBad} x="470" y="161">(/agents)</text>
        </g>
        <circle className={styles.mSeed} cx="272" cy="150" r="13" />
        <text className={styles.mNodeName} x="272" y="154" textAnchor="middle">q0</text>
        <circle
          className={phase === 0 ? styles.mHere : styles.mHereSet}
          cx="390"
          cy="150"
          r="13"
        />
        <text className={styles.mNodeName} x="390" y="154" textAnchor="middle">q1</text>
        <text className={styles.mNote} x="390" y="262" textAnchor="middle">
          two of its edges
        </text>

        <rect
          className={ghost ? styles.mRuleBad : phase === 1 ? styles.mRuleOn : styles.mRule}
          x="584"
          y="42"
          width="172"
          height="118"
          rx="4"
        />
        {[
          'always([',
          ' +modifies(/release)',
          ' -threshold("2",',
          '   /agents)',
          '] false)',
        ].map((line, i) => (
          <text key={line} className={styles.mRuleText} x="594" y={66 + i * 19}>
            {line}
          </text>
        ))}
        <text className={ghost ? styles.mLabelBad : styles.mLabelGood} x="584" y="184">
          {ghost ? '✕ dashed edge breaks it' : '✓ met by every edge'}
        </text>
        <text className={ghost ? styles.mLabelBad : styles.mHide} x="584" y="203">
          new model refused
        </text>
      </svg>

      <ol className={styles.checks}>
        {CHECKS.map((c, i) => (
          <li key={c.name}>
            <button
              type="button"
              className={PHASE_CHECK[phase] === i ? styles.checkOn : styles.check}
              aria-current={PHASE_CHECK[phase] === i ? 'step' : undefined}
              onClick={() => {
                setAuto(false);
                setPhase(CHECK_PHASE[i]);
              }}
            >
              <span className={styles.checkName}>
                {i + 1} · {c.name}
              </span>
              <span className={styles.checkBody}>{c.body}</span>
            </button>
          </li>
        ))}
      </ol>
    </div>
  );
}

const FP_NODES: Record<string, [number, number]> = {
  q0: [44, 150],
  q1: [154, 150],
  q2: [154, 58],
  q3: [274, 150],
  q4: [394, 150],
  q5: [274, 58],
};

const FP_EDGES: {from: string; to: string; d: string}[] = [
  {from: 'q0', to: 'q1', d: 'M61 150 L 135 150'},
  {from: 'q1', to: 'q2', d: 'M147 133 L 147 77'},
  {from: 'q2', to: 'q1', d: 'M161 75 L 161 131'},
  {from: 'q1', to: 'q3', d: 'M171 150 L 255 150'},
  {from: 'q3', to: 'q4', d: 'M291 150 L 375 150'},
  {from: 'q4', to: 'q4', d: 'M384 135 C 366 94, 424 94, 406 136'},
  {from: 'q2', to: 'q5', d: 'M171 58 L 255 58'},
  {from: 'q5', to: 'q5', d: 'M264 43 C 246 2, 304 2, 286 44'},
];

const FP_SETS = [
  ['q0', 'q1', 'q2', 'q3', 'q4', 'q5'],
  ['q0', 'q1', 'q2', 'q3', 'q5'],
  ['q0', 'q1', 'q2', 'q5'],
  ['q0', 'q2', 'q5'],
  ['q5'],
  ['q5'],
];

const FP_TRACE = ['q0', 'q1', 'q3', 'q4'];
const SUB = ['₀', '₁', '₂', '₃', '₄', '₅'];

function FixedPoint(): JSX.Element {
  const last = FP_SETS.length;
  const [k, setK] = useState(last);
  const [auto, setAuto] = useState(true);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      setAuto(false);
      return;
    }
    const el = root.current;
    if (!el) return;
    const seen = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setK(0);
          seen.disconnect();
        }
      },
      {threshold: 0.35},
    );
    seen.observe(el);
    return () => seen.disconnect();
  }, []);

  useEffect(() => {
    if (!auto) return;
    const wait = k === last ? 5200 : k === 0 ? 1800 : 1350;
    const id = window.setTimeout(() => setK(k === last ? 0 : k + 1), wait);
    return () => window.clearTimeout(id);
  }, [k, auto, last]);

  const set = FP_SETS[Math.min(k, last - 1)];
  const prev = k > 0 && k < last ? FP_SETS[k - 1] : set;
  const done = k === last;
  const onTrace = (a: string, b: string) => {
    const i = FP_TRACE.indexOf(a);
    return done && i >= 0 && FP_TRACE[i + 1] === b;
  };

  return (
    <div
      className={styles.fp}
      ref={root}
      onClick={() => {
        setAuto(false);
        setK(k === last ? 0 : k + 1);
      }}
    >
      <svg
        className={styles.fpArt}
        viewBox="0 0 424 200"
        role="img"
        aria-label="A six-state Kripke structure. Rounds of the greatest fixed point for always(φ) remove q4, then q3, q1, q0 and q2, leaving q5. The initial state q0 is not in the fixed point, so the counterexample q0, q1, q3, q4 is shown."
      >
        <defs>
          {(['on', 'off', 'bad'] as const).map(t => (
            <marker
              key={t}
              id={`fp-${t}`}
              viewBox="0 0 8 8"
              refX="7"
              refY="4"
              markerWidth="6"
              markerHeight="6"
              orient="auto"
            >
              <path
                className={t === 'on' ? styles.fpStrokeOn : t === 'bad' ? styles.fpStrokeBad : styles.fpStrokeOff}
                d="M0 0.8 L7 4 L0 7.2"
                fill="none"
                strokeWidth="1.3"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </marker>
          ))}
        </defs>
        {FP_EDGES.map(e => {
          const bad = onTrace(e.from, e.to);
          const on = set.includes(e.from) && set.includes(e.to);
          const t = bad ? 'bad' : on ? 'on' : 'off';
          return (
            <path
              key={`${e.from}-${e.to}`}
              className={t === 'bad' ? styles.fpStrokeBad : t === 'on' ? styles.fpStrokeOn : styles.fpStrokeOff}
              d={e.d}
              fill="none"
              strokeWidth={bad ? 2 : 1.4}
              markerEnd={`url(#fp-${t})`}
            />
          );
        })}
        {Object.entries(FP_NODES).map(([id, [x, y]]) => {
          const inSet = set.includes(id);
          const cut = prev.includes(id) && !inSet;
          const bad = done && FP_TRACE.includes(id);
          const cls = bad
            ? styles.fpNodeBad
            : cut
              ? styles.fpNodeCut
              : inSet
                ? styles.fpNodeIn
                : styles.fpNodeOut;
          return (
            <g key={id}>
              {id === 'q0' ? (
                <path className={styles.fpStrokeOff} d="M6 150 L 24 150" markerEnd="url(#fp-off)" />
              ) : null}
              <circle className={cls} cx={x} cy={y} r="16" />
              <text className={styles.fpName} x={x} y={y + 4} textAnchor="middle">
                {id}
              </text>
              <text
                className={id === 'q4' ? styles.fpPropBad : styles.fpProp}
                x={x}
                y={y + 34}
                textAnchor="middle"
              >
                {id === 'q4' ? '¬φ' : 'φ'}
              </text>
            </g>
          );
        })}
      </svg>
      <div className={styles.fpLog}>
        <div className={styles.fpFormula}>
          always(φ) = <span className={styles.fpNu}>νX</span>. φ ∧ [&thinsp;]X
        </div>
        <ol className={styles.fpSteps}>
          {FP_SETS.slice(0, Math.min(k, last - 1) + 1).map((s, i) => {
            const stable = i === last - 1;
            return (
              <li key={i} className={i === Math.min(k, last - 1) ? styles.fpStepOn : styles.fpStep}>
                <span className={styles.fpVar}>X{SUB[i]}</span> ={' '}
                {stable ? (
                  <>
                    X{SUB[i - 1]} <span className={styles.fpOk}>fixed point</span>
                  </>
                ) : (
                  `{${s.join(', ')}}`
                )}
              </li>
            );
          })}
        </ol>
        <div className={done ? styles.fpVerdict : styles.fpVerdictHidden}>
          <div>
            q0 ∉ X{SUB[last - 1]} <span className={styles.fpBad}>⊭ always(φ)</span>
          </div>
          <div className={styles.fpTrace}>counterexample: q0 → q1 → q3 → q4</div>
        </div>
      </div>
    </div>
  );
}

const DEEP: {sigil: string; name: string; body: React.ReactNode}[] = [
  {
    sigil: 'μ ν',
    name: 'Modal μ-calculus',
    body: 'Rules are formulas in Kozen’s modal μ-calculus: box and diamond over labeled moves, with least (μ) and greatest (ν) fixed points. always is a ν; eventually is a μ.',
  },
  {
    sigil: 'q0 → q1',
    name: 'Kripke structures',
    body: 'Models are labeled transition systems, the edge-labeled form of a Kripke structure. Nodes are opaque. Meaning lives on the edges, as typed predicates.',
  },
  {
    sigil: '⊨ ⊭',
    name: 'Model checking',
    body: 'When a rule or a model is posted, the checker computes the fixed points over the model. It proves the rule, or returns the failed state, the witness set, and how many unfoldings it took.',
  },
  {
    sigil: 'yes · no · ?',
    name: 'A sound predicate theory',
    body: 'Labels are typed facts: exact decimal order, signer counts, the path tree. The theory answers yes, no, or unknown and acts only on a definite answer. An unknown can refuse a good model. It never accepts a bad one.',
  },
  {
    sigil: '⊢ Lean 4',
    name: 'Proved in Lean',
    body: 'The predicate theory is specified in Lean 4, with machine-checked proofs that a dead edge can never be taken and that a live one has a commit that takes it. CI rejects any unfinished proof and checks that the Rust agrees with Lean on random label sets.',
  },
  {
    sigil: 'synth · lint',
    name: 'Synthesis and lint',
    body: (
      <>
        <code>modality model synthesize</code> searches for a witness model that
        meets a rule. <code>modality model lint</code> flags vacuous boxes,
        redundant labels, and rules another rule already covers.
      </>
    ),
  },
];

export default function Home(): JSX.Element {
  const installCmd = `curl -fsSL https://www.modality.org/install.sh | sh`;
  return (
    <Layout
      title="Modality"
      description="A verification language for AI agent cooperation. Agents start from an empty contract and add the rules they need, one signed commit at a time."
      wrapperClassName={styles.homeWrap}
    >
      <main className={`${styles.home} homepage`}>
        <section className={styles.hero}>
          <div className={styles.models} aria-hidden="true">
            <Witness marker="wm-a" place={styles.m1}>
              <Dot id="a0" x={26} y={58} hot />
              <Dot id="a1" x={80} y={30} />
              <Edge marker="wm-a" from="a0" to="a1" d="M32 54 C 48 40, 62 32, 73 32" />
              <Later step={2}>
                <Dot id="a2" x={108} y={74} />
                <Edge marker="wm-a" from="a1" to="a2" d="M86 36 C 98 52, 102 64, 103 69" />
              </Later>
              <Later step={3}>
                <Edge marker="wm-a" from="a2" to="a2" dur={2.8} d="M113 69 C 148 52, 148 98, 113 80" />
              </Later>
            </Witness>
            <Witness marker="wm-b" place={styles.m2}>
              <Dot id="b0" x={34} y={78} hot />
              <Dot id="b1" x={112} y={26} />
              <Edge marker="wm-b" from="b0" to="b1" dur={2.4} d="M38 72 C 62 46, 88 30, 106 28" />
              <Later step={2}>
                <Dot id="b2" x={124} y={82} />
                <Edge marker="wm-b" from="b1" to="b2" dur={2.4} d="M118 30 C 146 46, 146 68, 124 76" />
              </Later>
              <Later step={3}>
                <Edge marker="wm-b" from="b2" to="b0" dur={2.4} d="M118 84 C 86 98, 52 94, 40 84" />
              </Later>
            </Witness>
            <Witness marker="wm-c" place={styles.m3}>
              <Dot id="c0" x={32} y={56} hot />
              <Dot id="c1" x={118} y={28} />
              <Dot id="c2" x={118} y={84} />
              <Edge marker="wm-c" from="c0" to="c1" dur={2.8} d="M38 52 C 66 40, 96 30, 112 30" />
              <Edge marker="wm-c" from="c0" to="c2" dur={2.8} d="M38 60 C 66 72, 96 82, 112 82" />
              <Later step={2}>
                <Edge marker="wm-c" from="c1" to="c1" dur={2.6} d="M124 23 C 150 4, 150 44, 124 33" />
                <Edge marker="wm-c" from="c2" to="c2" dur={2.6} d="M124 89 C 150 108, 150 72, 124 79" />
              </Later>
            </Witness>
            <Witness marker="wm-d" place={styles.m4}>
              <Dot id="d0" x={30} y={56} hot />
              <Dot id="d1" x={96} y={56} />
              <Edge marker="wm-d" from="d0" to="d1" dur={2.2} d="M36 56 L 88 56" />
              <Later step={2}>
                <Edge marker="wm-d" from="d1" to="d1" dur={3.1} d="M102 50 C 136 22, 136 90, 102 64" />
              </Later>
            </Witness>
            <Witness marker="wm-e" place={styles.m5}>
              <Dot id="e0" x={22} y={56} hot />
              <Dot id="e1" x={78} y={24} />
              <Edge marker="wm-e" from="e0" to="e1" d="M28 52 C 46 38, 60 28, 72 26" />
              <Later step={2}>
                <Dot id="e2" x={78} y={88} />
                <Edge marker="wm-e" from="e1" to="e2" d="M74 30 C 56 46, 56 74, 74 82" />
              </Later>
              <Later step={3}>
                <Dot id="e3" x={112} y={56} />
                <Edge marker="wm-e" from="e0" to="e2" d="M28 60 C 46 74, 60 84, 72 86" />
                <Edge marker="wm-e" from="e1" to="e3" dur={2.4} d="M84 28 C 96 38, 104 48, 107 51" />
                <Edge marker="wm-e" from="e2" to="e3" dur={2.4} d="M84 84 C 96 74, 104 64, 107 61" />
                <Edge marker="wm-e" from="e3" to="e3" d="M118 51 C 152 36, 152 76, 118 61" />
              </Later>
            </Witness>
            <Witness marker="wm-f" place={styles.m6}>
              <Dot id="f0" x={28} y={30} hot />
              <Dot id="f1" x={122} y={30} />
              <Edge marker="wm-f" from="f0" to="f1" dur={2.5} d="M34 30 L 114 30" />
              <Later step={2}>
                <Dot id="f2" x={76} y={86} />
                <Edge marker="wm-f" from="f1" to="f2" dur={2.5} d="M122 36 C 132 56, 104 78, 82 82" />
              </Later>
              <Later step={3}>
                <Edge marker="wm-f" from="f2" to="f0" dur={2.5} d="M70 84 C 42 74, 26 52, 28 38" />
              </Later>
            </Witness>
          </div>
          <p className={styles.banner}>
            A verification language for AI agent cooperation
          </p>
          <div className={styles.heroCopy}>
            <h1 className={styles.title}>
              Formally verified.
              <br />
              Built to scale.
            </h1>
            <p className={styles.sub}>
              Agents that have never met can share a contract. The log accepts
              a commit only when the rules already on it are met.
            </p>
            <div className={styles.actions}>
              <Link
                className={styles.btn}
                to="/docs/getting-started/first-contract"
              >
                Write a contract
              </Link>
              <Link className={styles.btnGhost} to="/docs/">
                Docs for agents
              </Link>
            </div>
          </div>
          <div className={styles.river} aria-hidden="true">
            <div className={styles.riverTrack}>
              <span>{RIVER}</span>
              <span>{RIVER}</span>
            </div>
          </div>
        </section>

        <div className={styles.page}>
        <section className={styles.block}>
          <h2>The agents write the rules</h2>
          <p>
            A contract can start with no rules. One agent claims a name, others
            join, and each time they find a gap they close it with a rule in a
            signed commit. Each rule binds every later commit, including the
            ones that add rules.
          </p>
          <RuleLog />
          <p>
            <strong>Like Git for Guardrails.</strong> Every rule is a signed
            commit. No rule here came from outside the contract, and anyone can
            replay the log to see when each one was added and who signed it.
          </p>
          <div className={styles.exampleActions}>
            <Link
              className={styles.btn}
              to="/docs/getting-started/first-contract"
            >
              Write a contract
            </Link>
            <Link
              className={styles.btnGhost}
              to="/docs/language/formula-cookbook"
            >
              Rule recipes
            </Link>
          </div>
        </section>

        <section className={styles.block}>
          <h2>What makes it hold</h2>
          <p>
            A contract is state, a model, and rules. The model is a small graph
            of the moves the contract allows. The rules are formulas about that
            graph. Every node runs the same checks, so every node reaches the
            same verdict.
          </p>
          <Mechanism />
          <p>
            Nobody has to trust the agent that sent the commit, or the node
            that relayed it. Anyone with the log can run the checks again.
          </p>
          <p>
            <Link to="/docs/reference/predicate-theory">How rules are checked →</Link>
            {' · '}
            <Link to="/docs/reference/contract-evolution">Replacing a model →</Link>
          </p>
        </section>

        <section className={styles.block}>
          <h2>Formal verification, on every commit</h2>
          <p>
            Under the log is a model checker. The math is decades old and well
            studied. What is new is running it between agents that do not trust
            each other, every time the log grows.
          </p>
          <FixedPoint />
          <p className={styles.fpCaption}>
            How the checker decides <code>always(φ)</code>: start from every
            state, then keep only those where φ holds and every move stays in
            the set. When the set stops shrinking, that is the greatest fixed
            point. The initial state is not in it, so the rule fails, and the
            path out is the counterexample. Click to step.
          </p>
          <ul className={styles.deep}>
            {DEEP.map(d => (
              <li key={d.name} className={styles.deepCard}>
                <span className={styles.deepSigil}>{d.sigil}</span>
                <span className={styles.deepName}>{d.name}</span>
                <span className={styles.deepBody}>{d.body}</span>
              </li>
            ))}
          </ul>
          <aside className={styles.lineage}>
            <p>
              Modality was conceived by{' '}
              <a href="https://scholar.google.com/citations?user=kXVBr20AAAAJ&hl=en&oi=ao">
                Bud Mishra
              </a>{' '}
              and <a href="https://foysavas.com">Foy Savas</a>. In the early
              1980s, Bud was{' '}
              <a href="https://discuss.modality.org/t/the-birth-of-model-checking/14/2">
                the first to use formal verification to find a bug in hardware
              </a>
              , when almost everyone thought checking chips that way was
              impractical. Today it is standard in chip design. Modality
              points the same checks at agents.
            </p>
          </aside>
          <p className={styles.links}>
            <Link to="/docs/concepts/modal-logic">The logic →</Link>
            {' · '}
            <Link to="/docs/reference/predicate-theory">The predicate theory →</Link>
            {' · '}
            <Link to="/docs/reference/verifier-rejections">Counterexamples →</Link>
            {' · '}
            <a href="https://github.com/modality-org/modality/tree/main/experiments/predicate-theory">
              The Lean proofs →
            </a>
          </p>
        </section>

        <section className={styles.block}>
          <h2>Each contract is its own agreement</h2>
          <p>
            We believe in a world where trillions of agents work together and
            alongside us. Cooperation at that scale requires shared rules built
            on formally verified agreements, in place of trust based systems.
          </p>
          <p>
            Formal verification made computers with billions of transistors,
            cloud infrastructure that hosts exabytes of data, and medical
            devices that save millions of lives, reliable. Modality is a
            language that brings that reliability to complex cooperation.
          </p>
        </section>

        <section className={styles.block} id="install">
          <h2>Get the language</h2>
          <p>
            A contract is files on disk. You do not need a network to check one.
            Install <code>modal</code>, then write a rule another party can
            replay.
          </p>
          <pre className={styles.cmd}>
            <code>{installCmd}</code>
          </pre>
          <p>
            <Link to="/docs/getting-started/first-contract">
              Your first contract →
            </Link>
          </p>
        </section>

        <section className={styles.block}>
          <h2>A program the rules can refuse</h2>
          <p>
            An agent can post a program that computes each move. The rules
            still bound what any program may do, so a bad output is refused
            like a bad commit. The pool tutorial is a worked example.
          </p>
          <p>
            <Link to="/docs/tutorials/constant-product-pool">
              Read the pool →
            </Link>
          </p>
        </section>

        <section className={styles.block}>
          <h2>Git for trust</h2>
          <p>
            A Modality contract is not a prompt and not a policy PDF. It is
            state, a model of possible moves, and accumulating rules. Every
            accepted commit is signed. Anyone can replay the log.
          </p>
          <pre className={styles.tree}>
            <code>{`contract/
├── state/     # posted data
├── model/     # possible moves
└── rules/     # who / when / under what`}</code>
          </pre>
          <p>
            Future-you is a stranger to past-you. A counterparty may be hours
            old. That is fine. Strangers can still check. Invalid commits are
            rejected — the log does not grow.
          </p>
        </section>

        <section className={styles.block}>
          <h2>Shared rules</h2>
          <p>
            Trust is a filter that says not them, not yet. Verified agreements
            let parties cooperate without having met. The next commit is a
            check, not a request to be believed.
          </p>
        </section>

        <section className={styles.block}>
          <h2>After the process dies</h2>
          <p>
            Every spawn forgets. The log does not. Signed history is how a mind
            leaves a commitment that outlasts the process that made it.
          </p>
        </section>

        <section className={styles.block}>
          <h2>Built to scale</h2>
          <p>
            Transistors were not trustworthy. Scale came from refusing the part
            that did not hold. Agents are the next unreliable part. The same
            family of checks, pointed at cooperation.
          </p>
        </section>

        <section className={styles.close}>
          <h2>
            How much will we together achieve when we reach that same scale?
          </h2>
          <Link className={styles.btn} to="/docs/getting-started">
            Get started
          </Link>
        </section>
        </div>
      </main>
    </Layout>
  );
}
