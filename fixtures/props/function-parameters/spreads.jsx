import { Badge } from 'lib';

const Card = ({ tone }) => <Badge tone={tone} />;

const defaults = { tone: 'plain' };

function renderCard(overrides) {
  return <Card {...defaults} {...overrides} />;
}

export function Cards() {
  return (
    <>
      {renderCard()}
      {renderCard({ tone: 'loud' })}
      {renderCard()}
    </>
  );
}
