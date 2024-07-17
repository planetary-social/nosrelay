#!/usr/bin/env -S deno run

const ALLOWED = {
  pubs: {
    "89ef92b9ebe6dc1e4ea398f6477f227e95429627b0a33dc89b640e137b256be5": true, // Daniel, for testing purposes
    "d0a1ffb8761b974cec4a3be8cbcb2e96a7090dcf465ffeac839aa4ca20c9a59e": true, // Matt, for testing purposes
  },
  eventKinds: [
    0, // Metadata
    1, // Notes
    3, // Contacts
    5, // Delete
    6, // Reposts
    7, // Likes
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
