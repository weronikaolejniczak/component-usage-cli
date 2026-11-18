import { Icon, Button } from 'lib';

export const Wrapper = (props) => <Button {...props} />;

export const A = ({ open }) => (
  <>
    <Icon type="gear" />
    <Icon type={open ? 'arrowDown' : 'gear'} size="s" />
    <Icon type={name} />
    <Wrapper size="m" />
  </>
);
