import { Button } from 'lib';
import { ICONS, SIZE, Mutated, fromDeclare, missing } from './constants';
import * as all from './constants';
import icon from './default';
import { BARREL } from './barrel';
import { gone } from './nowhere';
import { cjs } from './cjs';
import data from './data.json';
import { self } from './cycle';
import { list } from './list';
import * as reexports from './barrel';
list.push('b');

export function App({ status }: { status: 'edit' | 'remove' }) {
  return (
    <Button
      iconType={ICONS.edit}
      type={ICONS[status]}
      size={SIZE}
      icon={all.ICONS.remove}
      name={all[status]}
      title={icon}
      label={BARREL}
      color={Mutated}
      fill={fromDeclare}
      aria-label={missing}
      prepend={gone}
      append={cjs}
      data={data}
      cycle={self}
      options={list}
      width={reexports.BARREL}
    />
  );
}
