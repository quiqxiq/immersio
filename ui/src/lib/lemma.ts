/**
 * Comprehensive English Lemmatization, Contraction & Inflection Engine
 * 
 * Accurately resolves:
 * 1. Contractions: they're -> they/be, didn't -> do/did, it's -> it/be, don't -> do, can't -> can, etc.
 * 2. Irregular Verbs: did/done/doing/does -> do, was/were/been -> be, went/gone -> go, thought -> think, etc.
 * 3. Irregular Plurals: children -> child, people -> person, women -> woman, teeth -> tooth, etc.
 * 4. Irregular Adjectives: better/best -> good, worse/worst -> bad, more/most -> many, etc.
 * 5. Possessives: world's -> world, today's -> today, students' -> student
 * 6. Regular inflections: -ies, -ied, -ing, -ed, -es, -s, doubled consonants (running -> run)
 * 7. Unicode apostrophe normalization: ’ and ‘ and ` -> '
 */

// 1. Contractions mapping to constituent lemma candidates
export const CONTRACTIONS_MAP: Record<string, string[]> = {
	// Negative contractions
	"didn't": ['do', 'did', 'not'],
	"didnt": ['do', 'did', 'not'],
	"don't": ['do', 'not'],
	"dont": ['do', 'not'],
	"doesn't": ['do', 'does', 'not'],
	"doesnt": ['do', 'does', 'not'],
	"won't": ['will', 'not'],
	"wont": ['will', 'not'],
	"can't": ['can', 'not'],
	"cant": ['can', 'not'],
	"cannot": ['can', 'not'],
	"couldn't": ['could', 'not'],
	"couldnt": ['could', 'not'],
	"shouldn't": ['should', 'not'],
	"shouldnt": ['should', 'not'],
	"wouldn't": ['would', 'not'],
	"wouldnt": ['would', 'not'],
	"wasn't": ['be', 'was', 'not'],
	"wasnt": ['be', 'was', 'not'],
	"weren't": ['be', 'were', 'not'],
	"werent": ['be', 'were', 'not'],
	"isn't": ['be', 'is', 'not'],
	"isnt": ['be', 'is', 'not'],
	"aren't": ['be', 'are', 'not'],
	"arent": ['be', 'are', 'not'],
	"haven't": ['have', 'not'],
	"havent": ['have', 'not'],
	"hasn't": ['have', 'has', 'not'],
	"hasnt": ['have', 'has', 'not'],
	"hadn't": ['have', 'had', 'not'],
	"hadnt": ['have', 'had', 'not'],
	"ain't": ['be', 'have', 'not'],
	"aint": ['be', 'have', 'not'],
	"mustn't": ['must', 'not'],
	"needn't": ['need', 'not'],
	"daren't": ['dare', 'not'],

	// Subject + To Be contractions
	"they're": ['they', 'be', 'are'],
	"theyre": ['they', 'be', 'are'],
	"it's": ['it', 'be', 'is'],
	"i'm": ['i', 'be', 'am'],
	"im": ['i', 'be', 'am'],
	"you're": ['you', 'be', 'are'],
	"youre": ['you', 'be', 'are'],
	"we're": ['we', 'be', 'are'],
	"were_contraction": ['we', 'be', 'are'], // handled in code to avoid collision with 'were'
	"he's": ['he', 'be', 'is'],
	"she's": ['she', 'be', 'is'],
	"that's": ['that', 'be', 'is'],
	"thats": ['that', 'be', 'is'],
	"there's": ['there', 'be', 'is'],
	"theres": ['there', 'be', 'is'],
	"here's": ['here', 'be', 'is'],
	"heres": ['here', 'be', 'is'],
	"what's": ['what', 'be', 'is'],
	"whats": ['what', 'be', 'is'],
	"who's": ['who', 'be', 'is'],
	"whos": ['who', 'be', 'is'],
	"where's": ['where', 'be', 'is'],
	"how's": ['how', 'be', 'is'],

	// Have contractions
	"i've": ['i', 'have'],
	"ive": ['i', 'have'],
	"you've": ['you', 'have'],
	"youve": ['you', 'have'],
	"we've": ['we', 'have'],
	"weve": ['we', 'have'],
	"they've": ['they', 'have'],
	"theyve": ['they', 'have'],
	"could've": ['could', 'have'],
	"should've": ['should', 'have'],
	"would've": ['would', 'have'],
	"might've": ['might', 'have'],
	"must've": ['must', 'have'],

	// Will contractions
	"i'll": ['i', 'will'],
	"ill": ['i', 'will'],
	"you'll": ['you', 'will'],
	"youll": ['you', 'will'],
	"he'll": ['he', 'will'],
	"she'll": ['she', 'will'],
	"we'll": ['we', 'will'],
	"they'll": ['they', 'will'],
	"that'll": ['that', 'will'],
	"there'll": ['there', 'will'],

	// Would / Had contractions
	"i'd": ['i', 'would', 'had'],
	"you'd": ['you', 'would', 'had'],
	"youd": ['you', 'would', 'had'],
	"he'd": ['he', 'would', 'had'],
	"she'd": ['she', 'would', 'had'],
	"we'd": ['we', 'would', 'had'],
	"they'd": ['they', 'would', 'had'],

	// Common spoken/colloquial forms
	"let's": ['let', 'us'],
	"lets": ['let', 'us'],
	"gonna": ['go', 'going'],
	"wanna": ['want'],
	"gotta": ['get', 'got'],
	"kinda": ['kind'],
	"sorta": ['sort'],
	"outta": ['out'],
	"gimme": ['give'],
	"lemme": ['let'],
	"cause": ['because'],
	"'cause": ['because'],
	"c'mon": ['come', 'on'],
	"y'all": ['you', 'all'],
	"ma'am": ['madam'],
	"o'clock": ['clock']
};

// 2. Comprehensive Irregular Verbs to Base Infinitive
export const IRREGULAR_VERBS_MAP: Record<string, string> = {
	// do
	did: 'do',
	done: 'do',
	doing: 'do',
	does: 'do',

	// be
	was: 'be',
	were: 'be',
	been: 'be',
	being: 'be',
	is: 'be',
	am: 'be',
	are: 'be',

	// have
	had: 'have',
	has: 'have',
	having: 'have',

	// go
	went: 'go',
	gone: 'go',
	going: 'go',
	goes: 'go',

	// say
	said: 'say',
	saying: 'say',
	says: 'say',

	// get
	got: 'get',
	gotten: 'get',
	getting: 'get',
	gets: 'get',

	// make
	made: 'make',
	making: 'make',
	makes: 'make',

	// know
	knew: 'know',
	known: 'know',
	knowing: 'know',
	knows: 'know',

	// think
	thought: 'think',
	thinking: 'think',
	thinks: 'think',

	// take
	took: 'take',
	taken: 'take',
	taking: 'take',
	takes: 'take',

	// see
	saw: 'see',
	seen: 'see',
	seeing: 'see',
	sees: 'see',

	// come
	came: 'come',
	coming: 'come',
	comes: 'come',

	// find
	found: 'find',
	finding: 'find',
	finds: 'find',

	// give
	gave: 'give',
	given: 'give',
	giving: 'give',
	gives: 'give',

	// tell
	told: 'tell',
	telling: 'tell',
	tells: 'tell',

	// feel
	felt: 'feel',
	feeling: 'feel',
	feels: 'feel',

	// become
	became: 'become',
	becoming: 'become',
	becomes: 'become',

	// leave
	left: 'leave',
	leaving: 'leave',
	leaves: 'leave',

	// put
	putting: 'put',
	puts: 'put',

	// mean
	meant: 'mean',
	meaning: 'mean',
	means: 'mean',

	// keep
	kept: 'keep',
	keeping: 'keep',
	keeps: 'keep',

	// let
	letting: 'let',
	lets: 'let',

	// begin
	began: 'begin',
	begun: 'begin',
	beginning: 'begin',
	begins: 'begin',

	// seem
	seemed: 'seem',
	seeming: 'seem',
	seems: 'seem',

	// help
	helped: 'help',
	helping: 'help',
	helps: 'help',

	// talk
	talked: 'talk',
	talking: 'talk',
	talks: 'talk',

	// turn
	turned: 'turn',
	turning: 'turn',
	turns: 'turn',

	// start
	started: 'start',
	starting: 'start',
	starts: 'start',

	// show
	showed: 'show',
	shown: 'show',
	showing: 'show',
	shows: 'show',

	// hear
	heard: 'hear',
	hearing: 'hear',
	hears: 'hear',

	// play
	played: 'play',
	playing: 'play',
	plays: 'play',

	// run
	ran: 'run',
	running: 'run',
	runs: 'run',

	// move
	moved: 'move',
	moving: 'move',
	moves: 'move',

	// like
	liked: 'like',
	liking: 'like',
	likes: 'like',

	// live
	lived: 'live',
	living: 'live',
	lives: 'live',

	// believe
	believed: 'believe',
	believing: 'believe',
	believes: 'believe',

	// hold
	held: 'hold',
	holding: 'hold',
	holds: 'hold',

	// bring
	brought: 'bring',
	bringing: 'bring',
	brings: 'bring',

	// write
	wrote: 'write',
	written: 'write',
	writing: 'write',
	writes: 'write',

	// provide
	provided: 'provide',
	providing: 'provide',
	provides: 'provide',

	// sit
	sat: 'sit',
	sitting: 'sit',
	sits: 'sit',

	// stand
	stood: 'stand',
	standing: 'stand',
	stands: 'stand',

	// lose
	lost: 'lose',
	losing: 'lose',
	loses: 'lose',

	// pay
	paid: 'pay',
	paying: 'pay',
	pays: 'pay',

	// meet
	met: 'meet',
	meeting: 'meet',
	meets: 'meet',

	// set
	setting: 'set',
	sets: 'set',

	// learn
	learnt: 'learn',
	learned: 'learn',
	learning: 'learn',
	learns: 'learn',

	// lead
	led: 'lead',
	leading: 'lead',
	leads: 'lead',

	// understand
	understood: 'understand',
	understands: 'understand',
	understanding: 'understand',

	// speak
	spoke: 'speak',
	spoken: 'speak',
	speaking: 'speak',
	speaks: 'speak',

	// spend
	spent: 'spend',
	spending: 'spend',
	spends: 'spend',

	// grow
	grew: 'grow',
	grown: 'grow',
	growing: 'grow',
	grows: 'grow',

	// win
	won: 'win',
	winning: 'win',
	wins: 'win',

	// buy
	bought: 'buy',
	buying: 'buy',
	buys: 'buy',

	// send
	sent: 'send',
	sending: 'send',
	sends: 'send',

	// build
	built: 'build',
	building: 'build',
	builds: 'build',

	// fall
	fell: 'fall',
	fallen: 'fall',
	falling: 'fall',
	falls: 'fall',

	// cut
	cutting: 'cut',
	cuts: 'cut',

	// sell
	sold: 'sell',
	selling: 'sell',
	sells: 'sell',

	// break
	broke: 'break',
	broken: 'break',
	breaking: 'break',
	breaks: 'break',

	// choose
	chose: 'choose',
	chosen: 'choose',
	choosing: 'choose',
	chooses: 'choose',

	// drive
	drove: 'drive',
	driven: 'drive',
	driving: 'drive',
	drives: 'drive',

	// eat
	ate: 'eat',
	eaten: 'eat',
	eating: 'eat',
	eats: 'eat',

	// catch
	caught: 'catch',
	catching: 'catch',
	catches: 'catch',

	// draw
	drew: 'draw',
	drawn: 'draw',
	drawing: 'draw',
	draws: 'draw',

	// fight
	fought: 'fight',
	fighting: 'fight',
	fights: 'fight',

	// throw
	threw: 'throw',
	thrown: 'throw',
	throwing: 'throw',
	throws: 'throw',

	// wear
	wore: 'wear',
	worn: 'wear',
	wearing: 'wear',
	wears: 'wear',

	// sleep
	slept: 'sleep',
	sleeping: 'sleep',
	sleeps: 'sleep',

	// fly
	flew: 'fly',
	flown: 'fly',
	flying: 'fly',
	flies: 'fly',

	// lie
	lay: 'lie',
	lain: 'lie',
	lying: 'lie',
	lies: 'lie',

	// rise
	rose: 'rise',
	risen: 'rise',
	rising: 'rise',
	rises: 'rise',

	// drink
	drank: 'drink',
	drunk: 'drink',
	drinking: 'drink',
	drinks: 'drink',

	// sing
	sang: 'sing',
	sung: 'sing',
	singing: 'sing',
	sings: 'sing',

	// swim
	swam: 'swim',
	swum: 'swim',
	swimming: 'swim',
	swims: 'swim',

	// hide
	hid: 'hide',
	hidden: 'hide',
	hiding: 'hide',
	hides: 'hide',

	// teach
	taught: 'teach',
	teaching: 'teach',
	teaches: 'teach',

	// blow
	blew: 'blow',
	blown: 'blow',
	blowing: 'blow',
	blows: 'blow',

	// ride
	rode: 'ride',
	ridden: 'ride',
	riding: 'ride',
	rides: 'ride',

	// wake
	woke: 'wake',
	woken: 'wake',
	waking: 'wake',
	wakes: 'wake',

	// forgive
	forgave: 'forgive',
	forgiven: 'forgive',
	forgiving: 'forgive',
	forgives: 'forgive',

	// forget
	forgot: 'forget',
	forgotten: 'forget',
	forgetting: 'forget',
	forgets: 'forget',

	// freeze
	froze: 'freeze',
	frozen: 'freeze',
	freezing: 'freeze',
	freezes: 'freeze',

	// shake
	shook: 'shake',
	shaken: 'shake',
	shaking: 'shake',
	shakes: 'shake',

	// shine
	shone: 'shine',
	shining: 'shine',
	shines: 'shine',

	// bite
	bit: 'bite',
	bitten: 'bite',
	biting: 'bite',
	bites: 'bite',

	// tear
	tore: 'tear',
	torn: 'tear',
	tearing: 'tear',
	tears: 'tear',

	// strike
	struck: 'strike',
	stricken: 'strike',
	striking: 'strike',
	strikes: 'strike',

	// hang
	hung: 'hang',
	hanging: 'hang',
	hangs: 'hang',

	// dig
	dug: 'dig',
	digging: 'dig',
	digs: 'dig',

	// feed
	fed: 'feed',
	feeding: 'feed',
	feeds: 'feed',

	// light
	lit: 'light',
	lighting: 'light',
	lights: 'light',

	// slide
	slid: 'slide',
	sliding: 'slide',
	slides: 'slide',

	// swear
	swore: 'swear',
	sworn: 'swear',
	swearing: 'swear',
	swears: 'swear',

	// sweep
	swept: 'sweep',
	sweeping: 'sweep',
	sweeps: 'sweep',

	// weep
	wept: 'weep',
	weeping: 'weep',
	weeps: 'weep',

	// steal
	stole: 'steal',
	stolen: 'steal',
	stealing: 'steal',
	steals: 'steal',

	// cost
	costing: 'cost',
	costs: 'cost',

	// hurt
	hurting: 'hurt',
	hurts: 'hurt',

	// hit
	hitting: 'hit',
	hits: 'hit',

	// shut
	shutting: 'shut',
	shuts: 'shut',

	// spread
	spreading: 'spread',
	spreads: 'spread',

	// split
	splitting: 'split',
	splits: 'split',

	// quit
	quitting: 'quit',
	quits: 'quit',

	// burn
	burnt: 'burn',

	// dream
	dreamt: 'dream',

	// bend
	bent: 'bend',

	// lend
	lent: 'lend',

	// bind
	bound: 'bind',

	// wind
	wound: 'wind',

	// seek
	sought: 'seek',

	// spin
	spun: 'spin',

	// sting
	stung: 'sting',

	// swing
	swung: 'swing',

	// kneel
	knelt: 'kneel',

	// creep
	crept: 'creep',

	// flee
	fled: 'flee',

	// bleed
	bled: 'bleed',

	// cling
	clung: 'cling',

	// fling
	flung: 'fling',

	// wring
	wrung: 'wring'
};

// 3. Irregular Plural Nouns to Singular
export const IRREGULAR_NOUNS_MAP: Record<string, string> = {
	children: 'child',
	people: 'person',
	men: 'man',
	women: 'woman',
	feet: 'foot',
	teeth: 'tooth',
	mice: 'mouse',
	geese: 'goose',
	oxen: 'ox',
	lives: 'life',
	leaves: 'leaf',
	wives: 'wife',
	knives: 'knife',
	halves: 'half',
	wolves: 'wolf',
	calves: 'calf',
	shelves: 'shelf',
	crises: 'crisis',
	theses: 'thesis',
	analyses: 'analysis',
	criteria: 'criterion',
	phenomena: 'phenomenon'
};

// 4. Irregular Adjectives / Adverbs
export const IRREGULAR_ADJECTIVES_MAP: Record<string, string> = {
	better: 'good',
	best: 'good',
	worse: 'bad',
	worst: 'bad',
	more: 'many',
	most: 'many',
	less: 'little',
	least: 'little',
	farther: 'far',
	further: 'far',
	furthest: 'far',
	farthest: 'far',
	older: 'old',
	oldest: 'old',
	elder: 'old',
	eldest: 'old'
};

/**
 * Normalize curly apostrophes, accents, and trim outer punctuation
 */
export function cleanWordToken(raw: string): string {
	if (!raw) return '';
	return raw
		.toLowerCase()
		.trim()
		.replace(/[\u2018\u2019`]/g, "'") // Convert curly quotes to standard ASCII apostrophe
		.replace(/^[^a-z0-9']+|[^a-z0-9']+$/g, ''); // Trim leading/trailing punctuation except apostrophe
}

/**
 * Returns an ordered array of candidate lemmas/stems to test against vocabulary statuses.
 * Candidate order prioritizes:
 * 1. The surface cleaned word itself (e.g. "didn't")
 * 2. Primary root/lemma (e.g. "do", "did")
 * 3. Base noun if possessive (e.g. "today's" -> "today")
 * 4. Regular stemmed forms (e.g. -ing, -ed, -s)
 */
export function getLemmaCandidates(rawWord: string): string[] {
	const clean = cleanWordToken(rawWord);
	if (!clean) return [];

	const candidates: string[] = [];
	const seen = new Set<string>();

	const add = (w: string) => {
		const c = cleanWordToken(w);
		if (c && !seen.has(c)) {
			seen.add(c);
			candidates.push(c);
		}
	};

	// 1. Always start with exact surface word
	add(clean);

	// 2. Contractions dictionary lookup
	if (CONTRACTIONS_MAP[clean]) {
		for (const target of CONTRACTIONS_MAP[clean]) {
			add(target);
		}
	}

	// 3. Possessives: e.g. "today's" -> "today", "students'" -> "student"
	if (clean.endsWith("'s") && clean.length > 2) {
		const base = clean.slice(0, -2);
		add(base);
	} else if (clean.endsWith("s'") && clean.length > 2) {
		const base = clean.slice(0, -2);
		add(base);
	}

	// 4. Irregular verbs lookup
	if (IRREGULAR_VERBS_MAP[clean]) {
		add(IRREGULAR_VERBS_MAP[clean]);
	}

	// 5. Irregular nouns lookup
	if (IRREGULAR_NOUNS_MAP[clean]) {
		add(IRREGULAR_NOUNS_MAP[clean]);
	}

	// 6. Irregular adjectives lookup
	if (IRREGULAR_ADJECTIVES_MAP[clean]) {
		add(IRREGULAR_ADJECTIVES_MAP[clean]);
	}

	// 7. Regular English Morphological Stemming Rules (skip for contractions or already resolved irregulars)
	if (!clean.includes("'") && !IRREGULAR_ADJECTIVES_MAP[clean] && !IRREGULAR_NOUNS_MAP[clean]) {
		const w = clean;

		// -ies -> -y (parties -> party, studies -> study)
		if (w.endsWith('ies') && w.length > 4) {
			add(w.slice(0, -3) + 'y');
		}

		// -ied -> -y (studied -> study, cried -> cry)
		if (w.endsWith('ied') && w.length > 4) {
			add(w.slice(0, -3) + 'y');
		}

		// -ves -> -f or -fe (wolves -> wolf, knives -> knife)
		if (w.endsWith('ves') && w.length > 4) {
			add(w.slice(0, -3) + 'f');
			add(w.slice(0, -3) + 'fe');
		}

		// -es after s, sh, ch, x, z (watches -> watch, boxes -> box)
		if (/(?:ch|sh|ss|x|z)es$/.test(w) && w.length > 4) {
			add(w.slice(0, -2));
		}

		// -ing (running -> run, making -> make, playing -> play)
		if (w.endsWith('ing') && w.length > 4) {
			const stem = w.slice(0, -3);
			// Doubled consonant (running -> run, stopping -> stop)
			if (/(.)\1$/.test(stem) && stem.length > 2) {
				add(stem.slice(0, -1));
			}
			add(stem);
			add(stem + 'e'); // e.g. making -> make, taking -> take
		}

		// -ed (stopped -> stop, moved -> move, played -> play)
		if (w.endsWith('ed') && w.length > 4) {
			const stem = w.slice(0, -2);
			// Doubled consonant (stopped -> stop, planned -> plan)
			if (/(.)\1$/.test(stem) && stem.length > 2) {
				add(stem.slice(0, -1));
			}
			add(stem);
			add(stem + 'e'); // e.g. loved -> love, moved -> move
		}

		// -er / -est comparative (faster -> fast, biggest -> big)
		if (w.endsWith('est') && w.length > 4) {
			const stem = w.slice(0, -3);
			if (/(.)\1$/.test(stem) && stem.length > 2) {
				add(stem.slice(0, -1));
			}
			add(stem);
			add(stem + 'e');
		} else if (w.endsWith('er') && w.length > 4) {
			const stem = w.slice(0, -2);
			if (/(.)\1$/.test(stem) && stem.length > 2) {
				add(stem.slice(0, -1));
			}
			add(stem);
			add(stem + 'e');
		}

		// -s (books -> book, plays -> play)
		if (w.endsWith('s') && !w.endsWith('ss') && !w.endsWith('us') && !w.endsWith('is') && w.length > 3) {
			add(w.slice(0, -1));
		}
	}

	return candidates;
}

/**
 * Returns the primary dictionary headword / base lemma for a word.
 * E.g.:
 * "did" -> "do"
 * "didn't" -> "do"
 * "they're" -> "they"
 * "it's" -> "it"
 * "children" -> "child"
 * "running" -> "run"
 * "better" -> "good"
 * "today's" -> "today"
 */
export function getPrimaryLemma(rawWord: string): string {
	const clean = cleanWordToken(rawWord);
	if (!clean) return '';

	// Contractions: return subject or primary action
	if (CONTRACTIONS_MAP[clean] && CONTRACTIONS_MAP[clean].length > 0) {
		return CONTRACTIONS_MAP[clean][0];
	}

	// Possessive: return base noun
	if (clean.endsWith("'s") && clean.length > 2) {
		return clean.slice(0, -2);
	}

	// Irregulars
	if (IRREGULAR_VERBS_MAP[clean]) {
		return IRREGULAR_VERBS_MAP[clean];
	}
	if (IRREGULAR_NOUNS_MAP[clean]) {
		return IRREGULAR_NOUNS_MAP[clean];
	}
	if (IRREGULAR_ADJECTIVES_MAP[clean]) {
		return IRREGULAR_ADJECTIVES_MAP[clean];
	}

	// Fallback to morphological root if candidates exist
	const candidates = getLemmaCandidates(clean);
	if (candidates.length > 1) {
		return candidates[1];
	}

	return clean;
}
