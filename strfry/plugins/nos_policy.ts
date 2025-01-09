import type { Policy } from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/nos-changes/mod.ts";

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
    "2a5ce82d946a0e086f9228f68494f3597e91510c66bd201b442c968cd8381502": true, // Pro Publica
    "68ac0f27c0545377ec6e7c5ce6aa2d6ef8aa1edadc6a8c2ffae8eda07f26affc": true, // Robert Reich
    "407069c625e86232ae5c5709a6d2c71ef8df24f61d3c57784ca5404cb10229a0": true, // Bill Bennet 
    "04d1fabc2623f568dc600d7ebb4ea1a13b8ccfdc2c5bca1d955f769f4562e82f": true, // The Spinoff
    "d4a5cb6ef3627f22a9ac5486716b8d4dc44270898ef16da75d4ba05754cdbdc5": true, // Dan Slevin
    "fd615dad65d0a6ee443f4e49c0da3e26a264f42ea67d694fdceb38e7abeceb28": true, // Lucire
    "696736ec91f9b497bf0480f73530abd5c4a3bf8e261cfb23096dd88297a2190f": true, // Taylor Lorenz
    "82acde23330b88e6831146a373eee2716c57df3e0054c5187169e92ee0880120": true, // Al Jazeera
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

  if (!isDisallowed && isAllowedEventKind && isAllowedPub) {
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
