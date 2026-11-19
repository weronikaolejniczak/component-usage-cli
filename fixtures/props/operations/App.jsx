import { Button } from 'lib';

const LIMIT = 10;

export function App({ count, items }) {
  return (
    <Button
      isDisabled={!items}
      isLoading={count > LIMIT}
      size={-count}
      width={count * 2 + 1}
      label={typeof count}
      tabIndex={~count}
      title={'a' in items}
      name={items instanceof Array}
      iconType={new Icon()}
      isSelected={!true}
      offset={-5}
    />
  );
}
