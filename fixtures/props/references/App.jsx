import { Button, Icon, Popover } from 'lib';
import * as L from 'lib';

const map = { gear: Icon, [key]: Button };
const list = [Icon, (L.Button)];
const Alias = Icon;

export const App = () => (
  <Popover button={Button} icon={cond ? Icon : null}>
    <Icon />
    {render(Button, { as: L.Icon })}
  </Popover>
);

Icon.displayName = 'x';
export function pick() {
  return Button;
}
export { Icon };
const fallback = other || Icon;
const shown = Icon && other;
const Wrapped = wrap(Button);
