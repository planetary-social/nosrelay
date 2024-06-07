#!/usr/bin/env -S deno run

const ALLOWED = {
  pubs: {
    "56d4b3d6310fadb7294b7f041aab469c5ffc8991b1b1b331981b96a246f6ae65": true, // Tagr
  },
  eventKinds: [
    0, // Metadata
    3, // Contacts
    1059, // Gift wrap messages
    10002, // Relay list metadata
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

  if (isAllowedPub || isAllowedEventKind) {
    res.action = "accept";
  } else {
    res.action = "reject";
    res.msg = "blocked: not on white-list";
  }

  console.log(JSON.stringify(res));
});
