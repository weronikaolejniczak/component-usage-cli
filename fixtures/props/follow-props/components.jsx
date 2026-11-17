import { memo, forwardRef } from 'react';
import { Button } from 'lib';

export function Header({ iconType = 'gear', ...rest }) {
  return <Button iconType={iconType} title={rest.title} color={rest.iconType} />;
}

export function Panel(props) {
  const { fill } = props;
  return <Button label={props.label} fill={fill} />;
}

export function Forward(props) {
  return <Button {...props} />;
}

export function Outer({ icon }) {
  return <Header iconType={icon} />;
}

export const Memo = memo(({ name }) => <Button name={name} />);

const Inner = ({ append }) => <Button append={append} />;
export const Wrapped = React.memo(Inner);

export function Escaped({ size }) {
  return <Button size={size} />;
}
export const Styled = styled(Escaped);

export function Unused({ width }) {
  return <Button width={width} />;
}

export function List({ items }) {
  return items.map((item) => <Button prepend={item.icon} />);
}

export function Tree({ depth }) {
  return (
    <Button depth={depth}>
      <Tree depth={depth} />
    </Button>
  );
}

export function Labeled({ children }) {
  return <Button aria-label={children} />;
}

export default ({ data }) => <Button data={data} />;

export function Rest({ icon, ...rest }) {
  return <Button {...rest} />;
}

export const Ref = forwardRef(({ size }, ref) => <Button size={size} />);
