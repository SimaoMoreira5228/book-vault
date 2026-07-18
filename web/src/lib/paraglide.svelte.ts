import type { Locale as _Locale } from "$lib/paraglide/runtime";
import { browser } from "$app/env";
import { goto } from "$app/navigation";
import { page } from "$app/state";

import {
	baseLocale,
	localizeUrl,
	overwriteGetLocale,
	overwriteSetLocale,
	toLocale
} from "$lib/paraglide/runtime";

export class Locale {
	#current: _Locale = $state(
		toLocale(browser && document.querySelector("html")?.lang) ?? baseLocale
	);

	constructor() {
		overwriteGetLocale(() => this.#current);

		overwriteSetLocale((locale) => {
			this.#current = locale;
			// eslint-disable-next-line svelte/no-navigation-without-resolve
			goto(localizeUrl(page.url.pathname, { locale }));
		});
	}
}
