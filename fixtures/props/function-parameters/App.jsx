import { Badge } from 'lib';
import { renderTag } from './helpers';

function renderBadge(color, { size = 's', ...rest }) {
  return <Badge color={color} size={size} title={rest.title} />;
}

const label = (text = 'none') => <Badge label={text} />;

function spread(...args) {
  return <Badge first={args} />;
}

function passed(value) {
  return <Badge value={value} />;
}

const PAIR = ['green', {}];

export function App({ items }) {
  return (
    <>
      {renderBadge('red', { size: 'l', title: 'One' })}
      {renderBadge('blue', {})}
      {renderBadge(...PAIR)}
      {label()}
      {label('hi')}
      {renderTag('x')}
      {items.map(passed)}
      {spread(1)}
    </>
  );
}
