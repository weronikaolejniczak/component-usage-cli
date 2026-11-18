import * as All from './barrel';
import { ui } from './barrel';

export const App = () => (
  <>
    <All.ui.Toned tone="x" />
    <ui.Toned tone="y" />
  </>
);
