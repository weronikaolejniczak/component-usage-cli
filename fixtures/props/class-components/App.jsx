import { Component } from 'react';
import { Badge, Button } from 'lib';

class Panel extends Component {
  static defaultProps = { size: 's' };
  state = { open: false };
  title = 'Panel';
  onClick = () => {};
  handler = null;
  constructor(props) {
    super(props);
    this.handler = this.toggle.bind(this);
  }
  toggle() {}
  render() {
    const { color } = this.props;
    return (
      <>
        <Button size={this.props.size} color={color} label={this.title} onClick={this.onClick} />
        <Button onToggle={this.toggle} isOpen={this.state.open} onBlur={this.handler} other={this.missing} />
        {this.props.items.map((item) => <Badge id={item.id} />)}
        {function () { return <Badge color={this.props.color} />; }}
      </>
    );
  }
}

function Tag({ size }) {
  return <Badge size={size} />;
}
Tag.defaultProps = { size: 'm' };

export function App() {
  return (
    <>
      <Panel color="red" items={[{ id: 1 }]} />
      <Panel size="l" color="blue" items={[]} />
      <Tag />
      <Tag size="xl" />
    </>
  );
}
