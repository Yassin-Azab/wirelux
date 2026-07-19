/**
 * Map/CountryLookup/index.js
 * Maps country names to topojson feature IDs (ISO 3166-1 numeric).
 * world-atlas uses numeric IDs that correspond to ISO 3166-1 numeric codes.
 */

// Country name → ISO numeric ID
// Covers ~250 countries. Extended as needed.
const NAME_TO_ID = {
  'Afghanistan': 4, 'Albania': 8, 'Algeria': 12, 'Andorra': 20,
  'Angola': 24, 'Antigua and Barbuda': 28, 'Argentina': 32,
  'Armenia': 51, 'Australia': 36, 'Austria': 40, 'Azerbaijan': 31,
  'Bahamas': 44, 'Bahrain': 48, 'Bangladesh': 50, 'Barbados': 52,
  'Belarus': 112, 'Belgium': 56, 'Belize': 84, 'Benin': 204,
  'Bhutan': 64, 'Bolivia': 68, 'Bosnia and Herzegovina': 70,
  'Botswana': 72, 'Brazil': 76, 'Brunei': 96, 'Bulgaria': 100,
  'Burkina Faso': 854, 'Burundi': 108, 'Cabo Verde': 132,
  'Cambodia': 116, 'Cameroon': 120, 'Canada': 124,
  'Central African Republic': 140, 'Chad': 148, 'Chile': 152,
  'China': 156, 'Colombia': 170, 'Comoros': 174, 'Congo': 178,
  'Democratic Republic of the Congo': 180, 'Costa Rica': 188,
  "Côte d'Ivoire": 384, 'Croatia': 191, 'Cuba': 192, 'Cyprus': 196,
  'Czechia': 203, 'Denmark': 208, 'Djibouti': 262, 'Dominica': 212,
  'Dominican Republic': 214, 'Ecuador': 218, 'Egypt': 818,
  'El Salvador': 222, 'Equatorial Guinea': 226, 'Eritrea': 232,
  'Estonia': 233, 'Eswatini': 748, 'Ethiopia': 231, 'Fiji': 242,
  'Finland': 246, 'France': 250, 'Gabon': 266, 'Gambia': 270,
  'Georgia': 268, 'Germany': 276, 'Ghana': 288, 'Greece': 300,
  'Grenada': 308, 'Guatemala': 320, 'Guinea': 324,
  'Guinea-Bissau': 624, 'Guyana': 328, 'Haiti': 332, 'Honduras': 340,
  'Hungary': 348, 'Iceland': 352, 'India': 356, 'Indonesia': 360,
  'Iran': 364, 'Iraq': 368, 'Ireland': 372, 'Israel': 376,
  'Italy': 380, 'Jamaica': 388, 'Japan': 392, 'Jordan': 400,
  'Kazakhstan': 398, 'Kenya': 404, 'Kiribati': 296, 'Kosovo': 383,
  'Kuwait': 414, 'Kyrgyzstan': 417, 'Laos': 418, 'Latvia': 428,
  'Lebanon': 422, 'Lesotho': 426, 'Liberia': 430, 'Libya': 434,
  'Liechtenstein': 438, 'Lithuania': 440, 'Luxembourg': 442,
  'Madagascar': 450, 'Malawi': 454, 'Malaysia': 458, 'Maldives': 462,
  'Mali': 466, 'Malta': 470, 'Marshall Islands': 584,
  'Mauritania': 478, 'Mauritius': 480, 'Mexico': 484,
  'Micronesia': 583, 'Moldova': 498, 'Monaco': 492, 'Mongolia': 496,
  'Montenegro': 499, 'Morocco': 504, 'Mozambique': 508,
  'Myanmar': 104, 'Namibia': 516, 'Nauru': 520, 'Nepal': 524,
  'Netherlands': 528, 'New Zealand': 554, 'Nicaragua': 558,
  'Niger': 562, 'Nigeria': 566, 'North Korea': 408,
  'North Macedonia': 807, 'Norway': 578, 'Oman': 512,
  'Pakistan': 586, 'Palau': 585, 'Panama': 591,
  'Papua New Guinea': 598, 'Paraguay': 600, 'Peru': 604,
  'Philippines': 608, 'Poland': 616, 'Portugal': 620, 'Qatar': 634,
  'Romania': 642, 'Russia': 643, 'Rwanda': 646,
  'Saint Kitts and Nevis': 659, 'Saint Lucia': 662,
  'Saint Vincent and the Grenadines': 670, 'Samoa': 882,
  'San Marino': 674, 'Sao Tome and Principe': 678,
  'Saudi Arabia': 682, 'Senegal': 686, 'Serbia': 688,
  'Seychelles': 690, 'Sierra Leone': 694, 'Singapore': 702,
  'Slovakia': 703, 'Slovenia': 705, 'Solomon Islands': 90,
  'Somalia': 706, 'South Africa': 710, 'South Korea': 410,
  'South Sudan': 728, 'Spain': 724, 'Sri Lanka': 144,
  'Sudan': 729, 'Suriname': 740, 'Sweden': 752, 'Switzerland': 756,
  'Syria': 760, 'Taiwan': 158, 'Tajikistan': 762, 'Tanzania': 834,
  'Thailand': 764, 'Timor-Leste': 626, 'Togo': 768, 'Tonga': 776,
  'Trinidad and Tobago': 780, 'Tunisia': 788, 'Turkey': 792,
  'Turkmenistan': 795, 'Tuvalu': 798, 'Uganda': 800, 'Ukraine': 804,
  'United Arab Emirates': 784, 'United Kingdom': 826,
  'United States': 840, 'Uruguay': 858, 'Uzbekistan': 860,
  'Vanuatu': 548, 'Venezuela': 862, 'Vietnam': 704, 'Yemen': 887,
  'Zambia': 894, 'Zimbabwe': 716,
  // Territories / Special
  'Hong Kong': 344, 'Macao': 446, 'Puerto Rico': 630,
  'Palestine': 275, 'Western Sahara': 732,
};

// Reverse map: ID → name (for tooltip lookups from topojson features)
const ID_TO_NAME = Object.fromEntries(
  Object.entries(NAME_TO_ID).map(([name, id]) => [id, name])
);

/**
 * Get the ISO numeric ID for a country name.
 * @param {string} name
 * @returns {number|null}
 */
export function countryNameToId(name) {
  if (!name) return null;
  // Exact match first
  if (NAME_TO_ID[name] !== undefined) return NAME_TO_ID[name];
  // Case-insensitive fallback
  const lower = name.toLowerCase();
  const found = Object.entries(NAME_TO_ID).find(([k]) => k.toLowerCase() === lower);
  return found ? found[1] : null;
}

/**
 * Get the country name for an ISO numeric ID.
 * @param {number|string} id
 * @returns {string|null}
 */
export function countryIdToName(id) {
  return ID_TO_NAME[+id] || null;
}

/**
 * Build a lookup map of id → topojson feature from loaded world data.
 * @param {object} countries  topojson FeatureCollection
 * @returns {Map<number, object>}
 */
export function buildFeatureMap(countries) {
  const map = new Map();
  for (const feature of countries.features) {
    map.set(+feature.id, feature);
  }
  return map;
}
