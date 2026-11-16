import { Button, Icon } from 'lib';
import { ICON } from './icons';

const local = 'gear';

export function App({ icon, item }: { icon: string; item: { icon: string } }) {
  return (
    <Button
      iconType={local}
      icon={ICON}
      type={icon}
      name={item.icon}
      title={ICONS[icon]}
      label={getLabel()}
      color={isOpen ? 'primary' : 'text'}
      size={size || 's'}
      fill={!isOpen}
      onClick={() => {}}
      prepend={<Icon type="plus" />}
      {...rest}
    />
  );
}

export class Old extends React.Component<{ type: string }> {
  render() {
    return <Icon type={this.props.type as string} />;
  }
}
