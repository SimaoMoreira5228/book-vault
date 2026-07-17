import * as v from "valibot";

export const LoginSchema = v.object({
	email: v.pipe(v.string(), v.nonEmpty("Email is required"), v.email("Invalid email address")),
	password: v.pipe(
		v.string(),
		v.nonEmpty("Password is required"),
		v.minLength(8, "Password must be at least 8 characters")
	)
});

export const RegisterSchema = v.object({
	displayName: v.pipe(
		v.string(),
		v.nonEmpty("Display name is required"),
		v.minLength(2, "Display name must be at least 2 characters"),
		v.maxLength(100, "Display name is too long")
	),
	email: v.pipe(v.string(), v.nonEmpty("Email is required"), v.email("Invalid email address")),
	password: v.pipe(
		v.string(),
		v.nonEmpty("Password is required"),
		v.minLength(8, "Password must be at least 8 characters"),
		v.maxLength(128, "Password is too long")
	)
});

export const CreateBookSchema = v.object({
	title: v.pipe(
		v.string(),
		v.nonEmpty("Title is required"),
		v.minLength(1, "Title is required"),
		v.maxLength(500, "Title is too long")
	)
});

export type LoginFormData = v.InferInput<typeof LoginSchema>;
export type RegisterFormData = v.InferInput<typeof RegisterSchema>;
export type CreateBookFormData = v.InferInput<typeof CreateBookSchema>;
