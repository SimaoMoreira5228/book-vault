import type { Handle } from "@sveltejs/kit";
import { baseLocale, overwriteGetLocale } from "$lib/paraglide/runtime";

overwriteGetLocale(() => baseLocale);

export const handle: Handle = ({ event, resolve }) => resolve(event);
