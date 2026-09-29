import React, {useEffect, useRef} from 'react';
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
      [...svg.querySelectorAll<SVGCircleElement>('[data-node]')];

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
      [...svg.querySelectorAll<SVGPathElement>('path[data-from]')].filter(
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

function TreasurySketch({joined}: {joined: boolean}): JSX.Element {
  return (
    <svg
      className={styles.sketch}
      viewBox="0 0 300 188"
      role="img"
      aria-hidden="true"
    >
      <line className={styles.spine} x1="22" y1="28" x2="22" y2="116" />
      <circle className={styles.node} cx="22" cy="28" r="3.5" />
      <circle className={styles.node} cx="22" cy="72" r="3.5" />
      <circle className={styles.node} cx="22" cy="116" r="3.5" />
      <rect className={styles.commit} x="40" y="12" width="220" height="32" rx="3" />
      <rect className={styles.commit} x="40" y="56" width="220" height="32" rx="3" />
      <circle className={styles.ring} cx="214" cy="72" r="5" />
      <circle className={styles.ring} cx="230" cy="72" r="5" />
      <rect className={styles.commit} x="40" y="100" width="220" height="32" rx="3" />
      {joined ? (
        <g className={styles.joining}>
          <line className={styles.spine} x1="22" y1="116" x2="22" y2="160" />
          <circle className={styles.nodeIn} cx="22" cy="160" r="3.5" />
          <rect className={styles.commitIn} x="40" y="144" width="220" height="32" rx="3" />
          <circle className={styles.keyIn} cx="214" cy="160" r="5" />
          <circle className={styles.keyIn} cx="230" cy="160" r="5" />
        </g>
      ) : (
        <g className={styles.halting}>
          <path className={styles.mark} d="M16 154 l12 12 M28 154 l-12 12" />
          <rect className={styles.commitOut} x="56" y="144" width="220" height="32" rx="3" />
          <circle className={styles.keyOut} cx="246" cy="160" r="5" />
        </g>
      )}
    </svg>
  );
}

export default function Home(): JSX.Element {
  const installCmd = `curl -fsSL https://www.modality.org/install.sh | sh`;
  const treasuryRule = `always([+modifies(/treasury) -threshold("2", /treasury)] false)`;
  const refusedCmd = `modal c commit
  --sign alice
  --path /treasury/withdrawals/0001.json
  --value '{"amount":100}'`;
  const acceptedCmd = `modal c commit
  --sign alice
  --sign bob
  --path /treasury/withdrawals/0001.json
  --value '{"amount":100}'`;

  return (
    <Layout
      title="Modality"
      description="A verification language for AI agent cooperation. One signature cannot move a treasury the rules say needs two."
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
              An agent can be told to send the funds. The log accepts a commit
              only when the rules already on it are met.
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
          <h2>One signature cannot spend it</h2>
          <p>
            Tell an agent to move the treasury. The rule is already on the
            log. A commit that writes under <code>/treasury</code> needs two
            of the keys there.
          </p>
          <div className={styles.caption}>rules/treasury.modality</div>
          <pre className={styles.cmd}>
            <code>{treasuryRule}</code>
          </pre>
          <div className={styles.pair}>
            <figure className={styles.scene}>
              <TreasurySketch joined={false} />
              <figcaption>
                <div className={styles.captionRefused}>Refused</div>
                <pre className={styles.cmd}>
                  <code>{refusedCmd}</code>
                </pre>
                <p className={styles.note}>One key. The log does not grow.</p>
              </figcaption>
            </figure>
            <figure className={styles.scene}>
              <TreasurySketch joined />
              <figcaption>
                <div className={styles.captionOk}>Accepted</div>
                <pre className={styles.cmd}>
                  <code>{acceptedCmd}</code>
                </pre>
                <p className={styles.note}>Two keys, on the same commit.</p>
              </figcaption>
            </figure>
          </div>
          <div className={styles.exampleActions}>
            <Link className={styles.btn} to="/docs/tutorials/multisig-treasury">
              Build the treasury
            </Link>
          </div>
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
          <h2>On the public testnet</h2>
          <p>
            The same check runs on the public testnet. It uses predicate
            theory v2: a signature is checked, and an edge whose labels cannot
            hold together is refused. This testnet is not mainnet.
          </p>
          <p>
            <a href="https://testnet.modality.network">Status</a>
            {' · '}
            <a href="https://node0.testnet.modality.network">Explorer</a>
            {' · '}
            <Link to="/docs/cli/join-testnet">Join the testnet →</Link>
          </p>
        </section>

        <section className={styles.block}>
          <h2>A program the rules can refuse</h2>
          <p>
            A constant-product pool computes each swap. The rules bound what
            any program may do: a payout goes to someone who paid in, and a
            swap never lowers the fee-adjusted product.
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
