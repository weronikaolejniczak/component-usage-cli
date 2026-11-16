import { Panel, Select } from 'lib';

const base = { padding: 4 };

export const App = () => (
  <Panel style={{ width: 200, ...base, 'min-height': '1em', [side]: 0, onClick() {} }}>
    <Select singleSelection={{ asPlainText: true }} options={[{ label: 'A', value: 'a' }, ...more]} />
  </Panel>
);
