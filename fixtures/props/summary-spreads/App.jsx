import { Button } from 'lib';

export const App = ({ rest }) => (
  <>
    <Button type="wrong" {...{ type: 'right' }} />
    <Button type="before" {...rest} />
    <Button {...{ type: 'first' }} type="second" />
    <Button {...{ size: 's' }} />
    <Button size="m" {...{ ...rest }} />
  </>
);
