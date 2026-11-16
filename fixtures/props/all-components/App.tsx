import Default, { Icon, Panel, useTheme } from 'lib';
import { Mine } from './mine';

export const App = () => {
  useTheme();
  return (
    <Panel>
      <Panel.Inner>
        <Icon />
        <Default />
        <Mine />
      </Panel.Inner>
    </Panel>
  );
};
