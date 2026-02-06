import type { VueWrapper } from '@vue/test-utils';
import { config } from '@vue/test-utils';

export function extraFindMethods(wrapper: VueWrapper) {
  return {
    findComponentByText(selector: Parameters<VueWrapper['findAllComponents']>['0'], text: string) {
      return wrapper.findAllComponents(selector).filter(element => element.text().trim() === text.trim())[0];
    },
    findElementByText(selector: Parameters<VueWrapper['findAll']>['0'], text: string) {
      return wrapper.findAll(selector).filter(element => element.text().trim() === text.trim())[0];
    },
  };
}

config.plugins.VueWrapper.install(extraFindMethods);
