import { Button } from 'lib';

const ICON = 'gear';
const ALIAS = ICON;
const ICONS = { edit: 'pencil', remove: 'trash' };
const SIZES = ['s', 'm'];
const { edit } = ICONS;
const { missing = 'empty' } = ICONS;
let mutable = 'a';
mutable = 'b';
const list = ['x'];
list.push('y');
function handle() {}
enum Kind {
  A = 'a',
}
const a = b;
const b = a;

export function App({ status, props }: { status: 'edit' | 'remove'; props: { icon: string } }) {
  const { icon } = props;
  const local = status ? 'eye' : ICON;
  return (
    <Button
      iconType={ALIAS}
      icon={ICONS.edit}
      type={ICONS[status]}
      size={SIZES[0]}
      name={edit}
      title={missing}
      aria-label={ICONS.nope}
      label={mutable}
      options={list}
      onClick={handle}
      color={Kind.A}
      fill={icon}
      prepend={local}
      css={css`width: ${SIZES[1]};`}
      cycle={a}
    />
  );
}
