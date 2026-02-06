import antfu from '@antfu/eslint-config';

export default antfu({
  stylistic: {
    quotes: 'single',
    semi: true,
  },
  vue: {
    overrides: {
      'vue/block-order': ['error', { order: ['script', 'template', 'style'] }],
      'vue/block-lang': ['error', { script: { lang: 'ts' }, style: { lang: 'css' } }],
      'vue/enforce-style-attribute': ['error', { allow: ['module'] }],

      'vue/max-attributes-per-line': ['error', { singleline: { max: 5 }, multiline: { max: 1 } }],
      'vue/first-attribute-linebreak': ['error', { singleline: 'ignore', multiline: 'below' }],
      'vue/html-indent': ['error', 2, { attribute: 1, baseIndent: 1, closeBracket: 0, alignAttributesVertically: false, ignores: [] }],
      'vue/singleline-html-element-content-newline': 'off',

      'vue/attributes-order': ['error', {
        order: ['DEFINITION', 'LIST_RENDERING', 'CONDITIONALS', 'RENDER_MODIFIERS', 'GLOBAL', 'UNIQUE', 'SLOT', 'TWO_WAY_BINDING', 'OTHER_DIRECTIVES', 'OTHER_ATTR', 'EVENTS', 'CONTENT'],
        alphabetical: false,
      }],
      'vue/attribute-hyphenation': ['error', 'always', { ignore: [] }],
      'vue/html-quotes': ['error', 'double'],

      'vue/define-emits-declaration': ['error', 'type-literal'],
      'vue/v-slot-style': ['error', { atComponent: 'shorthand', default: 'shorthand', named: 'shorthand' }],
    },
  },
});
