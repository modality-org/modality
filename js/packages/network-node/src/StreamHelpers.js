/* eslint-disable no-console */

import { pipe } from "it-pipe";
import * as lp from "it-length-prefixed";
import map from "it-map";
import { fromString as uint8ArrayFromString } from "uint8arrays/from-string";
import { toString as uint8ArrayToString } from "uint8arrays/to-string";

export function stdinToDuplex(stream, prefix = "") {
  // Read utf-8 from stdin
  process.stdin.setEncoding("utf8");
  pipe(
    // Read from stdin (the source)
    process.stdin,
    // Turn strings into buffers
    (source) =>
      map(source, (string) => uint8ArrayFromString(`${prefix}${string}`)),
    // Encode with length prefix (so receiving side knows how much data is coming)
    lp.encode(),
    // Write to the stream (the sink)
    stream.sink
  );
}

export function streamToConsole(stream, prefix = "") {
  pipe(
    // Read from the stream (the source)
    stream.source,
    // Decode length-prefixed data
    // lp.decode(),
    (source) => {
      console.log("source", source);
      return source;
    },
    // Turn buffers into strings
    (source) => map(source, (buf) => uint8ArrayToString(buf)),
    // Sink function

    async function (source) {
      // For each chunk of data
      for await (const msg of source) {
        // Output the data as a utf8 string
        console.log(prefix + msg.toString().replace("\n", ""));
      }
    }
  );
}

export function streamToString(stream, join = "", prefix = "") {
  pipe(
    // Read from the stream (the source)
    stream.source,
    // Decode length-prefixed data
    lp.decode(),
    // Turn buffers into strings
    (source) => map(source, (buf) => uint8ArrayToString(buf)),
    // Sink function
    async function (source) {
      // For each chunk of data
      for await (const msg of source) {
        // Output the data as a utf8 string
        console.log(prefix + msg.toString().replace("\n", ""));
      }
    }
  );
}
