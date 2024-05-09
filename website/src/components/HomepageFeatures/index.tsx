import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  description: JSX.Element;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'Isolated but Connected',
    description: (
      <>
        TODO
      </>
    ),
  },
  {
    title: 'Configurable',
    description: (
      <>
        TODO
      </>
    ),
  },
  {
    title: 'Built in Rust',
    description: (
      <>
        Faster than its predecessor Pythonic predecessor starknet-devnet, this baby is built with Rust (TM) to allow you the bestest possible user experience. Yasss!!!!
      </>
    ),
  },
];

function Feature({title, description}: FeatureItem) {
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
