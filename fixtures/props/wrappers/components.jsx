import { Button, Icon } from 'lib';

export function Spread(props) {
  return <Button {...props} />;
}
export const Rest = ({ size, ...rest }) => <Button {...rest} />;
export const Styled = wrap(Button)`color: red;`;
export const Chained = wrap(Button).with({ a: 1 })``;
export const Nested = (props) => <Spread {...props} />;
export const Named = ({ type }) => <Icon type={type} />;
export const Loop = (props) => <Loop {...props} />;
export const Both = (props) => (
  <>
    <Button {...props} />
    <Icon {...props} />
  </>
);
export default wrap(Icon);
export const Unused = wrap(Button);
