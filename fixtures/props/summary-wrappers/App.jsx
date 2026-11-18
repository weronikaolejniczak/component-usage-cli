import { Button } from 'lib';

const One = (props) => <Button {...props} />;
const Two = (props) => <Button fill="solid" {...props} />;

export const App = () => (
  <>
    <One size="s" />
    <Two size="m" />
    <Two size="m" />
  </>
);
