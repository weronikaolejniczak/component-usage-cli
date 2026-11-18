import { Icon } from 'lib';
import { Missing } from './missing';

let Reassigned = Icon;
Reassigned = Missing;

export const App = () => (
  <>
    <Reassigned />
    <Missing />
  </>
);
