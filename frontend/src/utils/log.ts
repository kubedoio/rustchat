const isDev = import.meta.env.DEV

function serialize(args: unknown[]): string {
  return JSON.stringify(args, (_key, value) => {
    if (value instanceof Error) {
      return { name: value.name, message: value.message, stack: value.stack }
    }
    return value
  })
}

export const log = {
  debug: (...args: unknown[]) => {
    if (isDev) console.log(serialize(args))
  },
  info: (...args: unknown[]) => {
    if (isDev) console.info(serialize(args))
  },
  warn: (...args: unknown[]) => {
    if (isDev) console.warn(serialize(args))
  },
  error: (...args: unknown[]) => {
    console.error(serialize(args))
  },
}
