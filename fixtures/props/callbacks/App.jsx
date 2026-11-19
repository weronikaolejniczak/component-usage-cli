import { Badge, Button } from 'lib';

const TAGS = ['a', 'b'];
const LINKS = [{ title: 'Docs', icon: 'help' }, { title: 'Home' }];

export function App({ items }) {
  return (
    <>
      {TAGS.map((tag, index) => <Badge color={tag} label={index} />)}
      {LINKS.map(({ title, icon = 'link' }) => <Button title={title} iconType={icon} />)}
      {items.map((item) => <Badge color={item.color} />)}
      {LINKS.map((link) => {
        link.seen = true;
        return <Badge color={link.title} />;
      })}
      {Children.map(items, (child) => <Badge color={child} />)}
    </>
  );
}
