import { Icon } from 'lib';

export const Imported = () => <Icon type="gear" />;

export const Shadowed = () => {
  const Icon = () => null;
  return <Icon />;
};
