module.exports = async () => {
  return {
    projects: ["<rootDir>/packages/*"],
    transform: {},
    testRegex: "/tests/*/.*\\.test\\.(ts|tsx)$",
    testPathIgnorePatterns: ["/node_modules/", "/dist/"],
    cacheDirectory: "./node_modules/.cache/jest",
    clearMocks: true,
    verbose: true,
    passWithNoTests: true,
  };
};
