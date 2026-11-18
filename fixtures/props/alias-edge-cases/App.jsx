import * as Lib from 'lib';
import { Button, Icon } from 'lib';

const Fallback = () => null;
const { Icon: WithDefault = Fallback } = Lib;
var Redeclared = Icon;
var Redeclared = Fallback;
const Wrapped = wrap((Button));
const tagged = Icon`x`;

export const App = () => (
  <>
    <WithDefault />
    <Redeclared />
    <Wrapped size="s" />
  </>
);
