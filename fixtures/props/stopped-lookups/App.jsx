import { Button } from 'lib';

function Field({ settings }) {
  return <Button id={settings.id} />;
}

export const Form = () => <Field settings={5} />;
