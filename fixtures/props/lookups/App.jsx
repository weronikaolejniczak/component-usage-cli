import { Button } from 'lib';

const BASE = { size: 's', color: 'blue' };
const STYLE = { ...BASE, color: 'red' };
const FIRST = ['a', 'b'];
const ALL = [...FIRST, 'c'];
const NESTED = [...[...FIRST], 'd'];
const KEYS = { [key]: 'x', size: 'm' };

export function App({ index }) {
  return (
    <Button
      size={STYLE.size}
      color={STYLE.color}
      type={ALL[2]}
      icon={ALL[1]}
      label={FIRST[index]}
      title={NESTED[0]}
      name={KEYS.size}
      id={KEYS.other}
    />
  );
}
