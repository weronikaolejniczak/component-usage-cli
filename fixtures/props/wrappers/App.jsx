import Default, { Spread, Rest, Styled, Chained, Nested, Named, Loop, Both } from './components';
import { Button } from 'lib';

export const App = () => (
  <>
    <Spread size="s" />
    <Rest size="m" color="primary" />
    <Styled onClick={go} />
    <Chained />
    <Nested size="l" />
    <Named type="gear" />
    <Loop a="1" />
    <Both b="2" />
    <Default type="x" />
    <Button>
      <Spread />
    </Button>
  </>
);
