import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import styles from './index.module.css';

const RIVER =
  'a verification language for AI agent cooperation   ·   negotiate and verify   ·   modal contracts   ·   append-only logs of signed commits   ·   prove commitments with temporal logic   ·   ';

export default function Home(): JSX.Element {
  const installCmd = `curl -fsSL https://www.modality.org/install.sh | sh`;

  return (
    <Layout
      title="Modality"
      description="A verification language for AI agent cooperation. Modality enables agents to negotiate and verify cooperation through formal verification."
      wrapperClassName={styles.homeWrap}
    >
      <main className={`${styles.home} homepage`}>
        <section className={styles.hero}>
          <svg
            className={styles.worlds}
            viewBox="0 0 1000 640"
            preserveAspectRatio="xMidYMid slice"
            aria-hidden="true"
          >
            <path
              className={styles.orbit}
              d="M 500 78
                 C 690 78 822 168 868 250
                 C 918 340 918 400 868 490
                 C 822 572 690 562 500 562
                 C 310 562 178 572 132 490
                 C 82 400 82 340 132 250
                 C 178 168 310 78 500 78 Z"
            />
            <circle className={styles.world} cx="500" cy="78" r="7" />
            <circle className={styles.world} cx="868" cy="250" r="7" />
            <circle className={styles.world} cx="868" cy="490" r="7" />
            <circle className={styles.world} cx="132" cy="250" r="7" />
            <circle className={styles.world} cx="132" cy="490" r="7" />
            <g className={styles.actualG} transform="translate(500 562)">
              <circle className={styles.actualRing} r="16" />
              <circle className={styles.actual} r="5" />
            </g>
            <circle className={styles.tokenStill} cx="500" cy="562" r="3.5" />
            <circle className={styles.token} r="3.5">
              <animateMotion
                dur="22s"
                repeatCount="indefinite"
                rotate="0"
                path="M 500 562
                      C 310 562 178 572 132 490
                      C 82 400 82 340 132 250
                      C 178 168 310 78 500 78
                      C 690 78 822 168 868 250
                      C 918 340 918 400 868 490
                      C 822 572 690 562 500 562"
              />
            </circle>
          </svg>
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
              Modality enables agents to negotiate and verify cooperation
              through formal verification. Define modal contracts as
              append-only logs of signed commits, and prove commitments with
              temporal logic.
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
          <h2>Trillions of agents, one checkable agreement</h2>
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
