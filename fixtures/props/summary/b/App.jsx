import { Icon } from 'lib';
import { Wrapper } from '../a/App';

const I = Icon;
let R = Icon;
R = null;

export const B = () => (
  <>
    <I type="gear" />
    <R />
    <Wrapper />
    <Other icon={Icon} />
  </>
);
