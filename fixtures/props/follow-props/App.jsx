import { Header, Panel, Forward, Outer, Memo, Wrapped, Escaped, List, Tree, Labeled, Rest, Ref, Card } from './components';
import * as C from './components';
import Anon from './components';

const SIZE = 's';

export function App() {
  return (
    <>
      <Header iconType="pencil" title="Edit" />
      <Header />
      <Panel label="one" fill={SIZE} />
      <C.Panel {...{ label: 'two' }} />
      <Forward iconType="bell" disabled />
      <Outer icon="trash" />
      <Memo name="m" />
      <Wrapped append="z" />
      <Escaped size="l" />
      <List items={[]} />
      <Tree depth={1} />
      <Labeled>text</Labeled>
      <Labeled children="c" />
      <Anon data="d" />
      <Rest icon="x" label="y" />
      <Ref size="s" />
      <Card title="t" />
    </>
  );
}
