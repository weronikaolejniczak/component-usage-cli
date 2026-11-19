import { Badge } from 'lib';

function List({ items }) {
  return items.map((item) => <Badge color={item.color} />);
}

export const App = () => <List items={[{ color: 'red' }, { color: 'blue' }]} />;
