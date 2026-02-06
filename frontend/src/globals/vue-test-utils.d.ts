import type { DOMWrapper } from '@vue/test-utils';

declare module '@vue/test-utils' {
  interface VueWrapper {
    findComponentByText: (selector: Parameters<VueWrapper['findAllComponents']>['0'], text: string) => DOMWrapper;
    findElementByText: (selector: Parameters<VueWrapper['findAll']>['0'], text: string) => DOMWrapper;
  }
}
