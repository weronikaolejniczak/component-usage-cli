import { Button } from 'lib';

export const Before = (props) => <Button size="s" {...props} />;
export const After = (props) => <Button {...props} fill="solid" />;
export const Toned = ({ tone, ...rest }) => <Button color={tone} {...rest} />;
export const Outer = (props) => <Toned tone="accent" {...props} label="x" />;
