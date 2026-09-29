import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import styles from './index.module.css';

const RIVER =
  'a verification language for AI agent cooperation   ·   negotiate and verify   ·   modal contracts   ·   append-only logs of signed commits   ·   prove commitments with temporal logic   ·   ';

function Witness({
  marker,
  place,
  children,
}: {
  marker: string;
  place: string;
  children: React.ReactNode;
}): JSX.Element {
  return (
    <svg
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
    </svg>
  );
}

function Dot({x, y, hot}: {x: number; y: number; hot?: boolean}): JSX.Element {
  return <circle className={hot ? styles.seed : styles.dot} cx={x} cy={y} r="5.5" />;
}

function Edge({d, marker}: {d: string; marker: string}): JSX.Element {
  return <path className={styles.edge} d={d} markerEnd={`url(#${marker})`} />;
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
              <Dot x={26} y={58} hot />
              <Dot x={80} y={30} />
              <Dot x={132} y={76} />
              <Edge marker="wm-a" d="M32 54 C 48 40, 62 32, 73 32" />
              <Edge marker="wm-a" d="M86 34 C 102 46, 116 60, 126 70" />
            </Witness>
            <Witness marker="wm-b" place={styles.m2}>
              <Dot x={34} y={78} hot />
              <Dot x={112} y={26} />
              <Dot x={124} y={82} />
              <Edge marker="wm-b" d="M38 72 C 62 46, 88 30, 106 28" />
              <Edge marker="wm-b" d="M118 30 C 146 46, 146 68, 124 76" />
              <Edge marker="wm-b" d="M118 84 C 86 98, 52 94, 40 84" />
            </Witness>
            <Witness marker="wm-c" place={styles.m3}>
              <Dot x={32} y={56} hot />
              <Dot x={124} y={26} />
              <Dot x={124} y={86} />
              <Edge marker="wm-c" d="M38 52 C 68 40, 98 30, 116 28" />
              <Edge marker="wm-c" d="M38 60 C 68 72, 98 82, 116 84" />
            </Witness>
            <Witness marker="wm-d" place={styles.m4}>
              <Dot x={30} y={56} hot />
              <Dot x={96} y={56} />
              <Edge marker="wm-d" d="M36 56 L 88 56" />
              <Edge marker="wm-d" d="M102 50 C 136 22, 136 90, 102 64" />
            </Witness>
            <Witness marker="wm-e" place={styles.m5}>
              <Dot x={22} y={56} hot />
              <Dot x={78} y={24} />
              <Dot x={78} y={88} />
              <Dot x={136} y={56} />
              <Edge marker="wm-e" d="M28 52 C 46 38, 60 28, 72 26" />
              <Edge marker="wm-e" d="M28 60 C 46 74, 60 84, 72 86" />
              <Edge marker="wm-e" d="M84 26 C 102 34, 118 44, 128 52" />
              <Edge marker="wm-e" d="M84 86 C 102 78, 118 68, 128 60" />
            </Witness>
            <Witness marker="wm-f" place={styles.m6}>
              <Dot x={28} y={30} hot />
              <Dot x={122} y={30} />
              <Dot x={76} y={86} />
              <Edge marker="wm-f" d="M34 30 L 114 30" />
              <Edge marker="wm-f" d="M122 36 C 132 56, 104 78, 82 82" />
              <Edge marker="wm-f" d="M70 84 C 42 74, 26 52, 28 38" />
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
