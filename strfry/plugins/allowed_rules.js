#!/usr/bin/env -S deno run

const ALLOWED = {
  pubs: {
    "b43cdcbe1b5a991e91636c1372abd046ff1d6b55a17722a2edf2d888aeaa3150": true, // NPD Media
    "9561cd80e1207f685277c5c9716dde53499dd88c525947b1dde51374a81df0b9": true, // RevolutionZ Podcast
    "e526964aad10b63c24b3a582bfab4ef5937c559bfbfff3c18cb8d94909598575": true, // MuckRock Foundation
    "36de364c2ea2a77f2ed42cd7f2528ef547b6ab0062e3645046188511fe106403": true, // ZNet
    "99d0c998eaf2dbfaead9abf50919eba6495d8d615f0ded6b320948a4a4f8c478": true, // Patrick Boehler
    "715dc06230d7c6aa62b044a8a764728ae6862eb100f1800ef91d5cc9f972dc55": true, // We Distribute
    "e70d313e00d3d77c3ca7324c082fce9bbdefbe1b88cf39d4e48078c1573808ed": true, // The Conversation
    "0403c86a1bb4cfbc34c8a493fbd1f0d158d42dd06d03eaa3720882a066d3a378": true, // Global Sports Center
    "a78363acf392e7f6805d9d87654082dd83a02c6c565c804533e62b6f1da3f17d": true, // Alastair Thompson
    "b5ad453f5410107a61fde33b0bf7f61832e96b13f8fd85474355c34818a34091": true, // The 74
  },
  eventKinds: [
    0, // Metadata
    1, // Notes
    3, // Contacts
    5, // Delete
    6, // Reposts
    7, // Likes
    62, // Vanish request
    1984, // Notes
    1059, // Gift wrap messages
    10002, // Relay list metadata
    30023, // Long-form
  ],
};

import { stdin, stdout } from "node:process";
import { createInterface } from "node:readline";

const rl = createInterface({
  input: stdin,
  output: stdout,
  terminal: false,
});

rl.on("line", (line) => {
  let req = JSON.parse(line);

  if (req.type !== "new") {
    return;
  }

  let res = { id: req.event.id }; // must echo the event's id

  const isAllowedPub = ALLOWED.pubs.hasOwnProperty(req.event.pubkey);
  const isAllowedEventKind = ALLOWED.eventKinds.includes(req.event.kind);

  if (isAllowedPub && isAllowedEventKind) {
    res.action = "accept";
  } else {
    res.action = "reject";
    res.msg = "blocked: not on allow-list";
  }

  console.log(JSON.stringify(res));
});
