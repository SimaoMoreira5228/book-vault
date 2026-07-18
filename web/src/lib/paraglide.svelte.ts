import { browser } from "$app/environment";
import { baseLocale, overwriteGetLocale, overwriteSetLocale } from "$lib/paraglide/runtime";

type AppLocale = "en" | "pt-PT";

export class Locale {
	#current = $state<AppLocale>(baseLocale as AppLocale);

	constructor() {
		if (!browser) return;
		const saved = localStorage.getItem("PARAGLIDE_LOCALE");
		if (saved === "en" || saved === "pt-PT") {
			this.#current = saved;
		}
		overwriteGetLocale(() => this.#current);
		overwriteSetLocale((l) => {
			this.#current = l;
			localStorage.setItem("PARAGLIDE_LOCALE", l);
		});
	}
}
