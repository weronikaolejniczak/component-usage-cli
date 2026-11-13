// Every component imported from LIBRARY in CHECKOUT, with props, written to OUT.
const escape = (text) => text.replace(/[.*+?^${}()|[\]\\/]/g, '\\$&');

module.exports = {
  crawlFrom: process.env.CHECKOUT,
  includeSubComponents: true,
  importedFrom: new RegExp(`^${escape(process.env.LIBRARY)}$`),
  globs: ['**/*.@(js|jsx|ts|tsx|mjs|cjs|mts|cts)'],
  exclude: (dir) => ['node_modules', '.git', 'target', 'build'].includes(dir),
  processors: [['raw-report', { outputTo: process.env.OUT }]],
};
