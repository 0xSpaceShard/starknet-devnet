import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  description: JSX.Element;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'Isolated but connected 🖥️',
    description: (
      <>
        Simulate{' '}
        <a
          href="https://docs.starknet.io/documentation/"
          target="_blank"
          rel="noopener noreferrer"
        >
          Starknet
        </a>{' '}
        in the comfort of your local network. Fork mainnet/testnet to interact
        with real-world smart contracts, while maintaining isolation.
      </>
    ),
  },
  {
    title: 'Configurable and preservable 🔧',
    description: (
      <>
        Gas price, predeployed accounts, chain ID... All of this and more can be
        configured according to your needs. Once your work is done, dump Devnet
        into a file and later load it to continue where you left off.
      </>
    ),
  },
  {
    title: 'Built in Rust 🦀',
    description: (
      <>
        Unlike its{' '}
        <a
          href="https://0xspaceshard.github.io/starknet-devnet/"
          target="_blank"
          rel="noopener noreferrer"
        >
          Pythonic predecessor
        </a>
        , this program is built in Rust to ensure a better performance and
        overall user experience.
      </>
    ),
  },
];

function Feature({ title, description }: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): JSX.Element {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
