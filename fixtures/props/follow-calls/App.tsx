import { Button } from 'lib';

const ICONS = ['gear'];
function compute() {
  const inner = () => {
    return 'n';
  };
  return 'c';
}
const styles = makeStyles('x');

export function App() {
  return (
    <Button
      a={getIcon('edit')}
      b={useMemo(() => ['m'], [])}
      c={ICONS.map((icon) => icon)}
      d={pick(function () {
        if (ok) return 'x';
        return;
      })}
      e={load(async () => 'y')}
      f={run(compute)}
      g={useMemo(() => ICONS, [ICONS])}
      h={api?.get(...keys)}
      i={styles.panel}
      j={make()[key]}
      k={() => 'z'}
    />
  );
}
function Inner({ size }) {
  return <Button l={sized(size)} />;
}
export const Page = () => <Inner size="s" />;
