import type { Policy } from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/nos-changes/mod.ts";

const ALLOWED = {
  pubs: {
    "56d4b3d6310fadb7294b7f041aab469c5ffc8991b1b1b331981b96a246f6ae65": true, // Tagr
  },
  eventKinds: [
    0, // Metadata
    1, // Short Text Note
    3, // Contacts
    4, // Encrypted Direct Messages
    5, // Event deletion
    6, // Repost
    7, // Reaction
    62, // Request to Vanish
    1059, // Gift wrap messages
    1984, // Reports
    10000, // Mute list
    10002, // Relay list metadata
    30000, // Follow Set
    30023, // Long-form Content
  ],
};

// This overrides the allowed rules above
const DISALLOWED = {
  pubs: {},
  startWithTexts: ["GM from ws"],
};

const nosPolicy: Policy<void> = (msg) => {
  const event = msg.event;
  const content = event.content;
  let res = {
    id: event.id,
    action: "reject",
    msg: "blocked: not authorized",
  };

  const isAllowedPub = ALLOWED.pubs.hasOwnProperty(event.pubkey);
  const isAllowedEventKind = ALLOWED.eventKinds.includes(event.kind);
  const isDisallowed =
    DISALLOWED.pubs.hasOwnProperty(event.pubkey) ||
    DISALLOWED.startWithTexts.some((text) => content.startsWith(text));

  if (!isDisallowed && (isAllowedEventKind || isAllowedPub)) {
    res.action = "accept";
    res.msg = "";

    return res;
  }

  if (!isAllowedEventKind) {
    res.msg = "blocked: kind not allowed";
  }

  return res;
};

export default nosPolicy;
