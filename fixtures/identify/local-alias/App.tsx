import { Icon } from 'lib';
import * as Lib from 'lib';
import { Icon as OtherIcon } from 'other-lib';

const Aliased = Icon;
const { Icon: Destructured } = Lib;
const Namespace = Lib;
const Chained = Aliased;
const Other = OtherIcon;

export const A = () => <Aliased />;
export const B = () => <Destructured />;
export const C = () => <Namespace.Icon />;
export const D = () => <Chained />;
export const E = () => <Other />;
export const F = () => <Icon />;
let Reassigned = Icon;
Reassigned = OtherIcon;
const Computed = Lib['Icon'];
const Optional = Lib?.Icon;
export const G = () => <Reassigned />;
export const H = () => <Computed />;
export const I = () => <Optional />;
let Kept = Icon;
const Mut = Lib;
Mut.Icon = OtherIcon;
export const J = () => <Kept />;
export const K = () => <Mut.Icon />;
