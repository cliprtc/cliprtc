import antfu from '@antfu/eslint-config'

export default antfu({
  react: true,
  rules: {
    'no-console': 'warn',
    'curly': ['error', 'multi-line', 'consistent'],
  },
})
