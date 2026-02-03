import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import CodeBlock from '@theme/CodeBlock';
import styles from './index.module.css';

export default function Home(): JSX.Element {
  const installCmd = `curl --proto '=https' --tlsv1.2 -sSf https://www.modality.org/install.sh | sh`;

  return (
    <Layout
      title="Modality"
      description="A verification language for AI agent cooperation"
    >
      <main className={styles.main}>
        <div className={styles.warning}>
          ⚠️ <strong>Work in Progress</strong> — Modality is under active development. APIs and syntax may change.
        </div>
        <div className={styles.hero}>
          <h1 className={styles.title}>Modality</h1>
          <p className={styles.tagline}>
            A verification language for AI agent cooperation
          </p>
          <p className={styles.description}>
            Modality enables agents to negotiate and verify cooperation through formal verification.
            Define modal contracts as append-only logs of signed commits, and prove commitments with temporal logic.
          </p>
        </div>

        <div className={styles.install}>
          <h2>Install</h2>
          <CodeBlock language="bash">
            {installCmd}
          </CodeBlock>
        </div>

        <div className={styles.features}>
          <div className={styles.feature}>
            <h3>🔐 Modal Contracts</h3>
            <p>
              Define state machines with temporal logic formulas that are formally verified.
              Know your contracts are correct before deployment.
            </p>
          </div>
          <div className={styles.feature}>
            <h3>🤝 Agent Cooperation</h3>
            <p>
              Enable AI agents to make binding commitments to each other.
              Escrow, swaps, milestones — all provably enforced.
            </p>
          </div>
          <div className={styles.feature}>
            <h3>📜 Append-Only Logs</h3>
            <p>
              Contracts are append-only logs of signed commits.
              Full history, transparent state, cryptographic integrity.
            </p>
          </div>
        </div>

        <div className={styles.cta}>
          <Link className="button button--primary button--lg" to="/docs/getting-started">
            Get Started →
          </Link>
          <Link className="button button--secondary button--lg" to="/docs/concepts">
            Learn More
          </Link>
        </div>

        <p className={styles.footnote}>
          * Humans are also welcome to use Modality, if they're sufficiently motivated.
        </p>
      </main>
    </Layout>
  );
}
