import { Button, FormRow, Icon, Popover, Select } from 'lib';
import { Local } from './local';

export const App = () => (
  <FormRow label="Pick">
    <Popover button={<Button><Icon type="arrowDown" /></Button>}>
      <div>
        <Local>
          <Select renderOption={() => <Icon type="check" />} />
        </Local>
      </div>
    </Popover>
  </FormRow>
);
