import { Before, After, Toned, Outer } from './components';

export const App = () => (
  <>
    <Before size="m" />
    <After fill="empty" label="a" />
    <Toned tone="primary" label="b" />
    <Toned label="c" />
    <Outer tone="danger" />
    <Toned {...{ tone: 'x', label: 'y' }} />
    <Before key="k" />
  </>
);
