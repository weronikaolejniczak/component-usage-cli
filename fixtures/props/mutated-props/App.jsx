import { Icon } from 'lib';

function Menu({ iconType }) {
  iconType = iconType || 'gear';
  return <Icon type={iconType} />;
}

function Other(props) {
  props.size = 'l';
  return <Icon size={props.size} />;
}

const Reassigned = (props) => {
  props = { type: 's' };
  return <Icon {...props} />;
};

export const App = () => (
  <>
    <Menu iconType="bolt" />
    <Other size="s" />
    <Reassigned type="x" />
  </>
);
