import { dirname } from 'dirname-filename-esm';

import setupEnv from "@thylacine-js/common/setupEnv.mjs";
import serve from "@thylacine-js/webapp/serve.mjs";

export default async function main({ port = 3000, datastore }) {
  process.env.WEBAPP_DIR = './app/';
  process.env.APP_PROTOCOL = 'http';
  const __dirname = dirname(import.meta);
  await setupEnv();
  if (port) {
    process.env.APP_PORT = port;
    process.env.API_PORT = port+1;
  }
  process.env.API_ORIGIN = `http://0.0.0.0:${process.env.API_PORT}`;
  await serve(__dirname);
};

import cliCalls from 'cli-calls';
await cliCalls(import.meta, main)