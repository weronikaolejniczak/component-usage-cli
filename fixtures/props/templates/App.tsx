import { css } from 'styles';
import { Panel } from 'lib';

const color = 'red';

export const App = () => (
  <Panel
    css={css`
      color: ${color};
    `}
    className={`panel ${color}-text`}
    title={`${'a'}`}
  />
);
