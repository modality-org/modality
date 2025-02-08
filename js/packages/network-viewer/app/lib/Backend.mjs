import "isomorphic-fetch";
import "isomorphic-form-data";
// import { FormData } from 'formdata-polyfill/esm.min.js';
import { serialize } from "@shoelace-style/shoelace/dist/utilities/form.js";

const DEFAULT_HEADERS = {
  credentials: "include",
};

export function serializeForm(form) {
  return serialize(form);
}

export function urlFor(path) {
  return `${process.env.API_ORIGIN}${path}`;
}

export function headersFor(headers) {
  return Object.assign({}, DEFAULT_HEADERS, headers);
}

export function get(path) {
  return fetch(urlFor(path), headersFor())
    .then((response) => {
      return response.json().then((json) => ({ response, json }));
    })
    .then(({ response, json }) => {
      if (!response.ok) {
        return Promise.reject(response);
      }
      if (json.errors?.find((i) => i.status === 404)) {
        return (window.location.href = "/not-found");
      }
      if (!json.ok) {
        return Promise.reject(json);
      }
      return json;
    });
}

export function post(path, params) {
  let body = new FormData();
  if (params instanceof FormData) {
    body = params;
  } else {
    for (let key in params) {
      body.append(key, params[key]);
    }
  }
  return fetch(urlFor(path), headersFor({ method: "post", body }))
    .then((response) => {
      return response.json().then((json) => ({ response, json }));
    })
    .then(({ response, json }) => {
      if (!response.ok) {
        return Promise.reject(response);
      }
      if (!json.ok) {
        return Promise.reject(json);
      }
      return json;
    });
}

export function put(path, params) {
  let body = new FormData();
  if (params instanceof FormData) {
    body = params;
  } else {
    for (let key in params) {
      body.append(key, params[key]);
    }
  }

  return fetch(urlFor(path), headersFor({ method: "put", body }))
    .then((response) => {
      return response.json().then((json) => ({ response, json }));
    })
    .then(({ response, json }) => {
      if (!response.ok) {
        return Promise.reject(response);
      }
      if (!json.ok) {
        return Promise.reject(json);
      }
      return json;
    });
}

export function del(path) {
  return fetch(urlFor(path), headersFor({ method: "delete" }))
    .then((response) => {
      return response.json().then((json) => ({ response, json }));
    })
    .then(({ response, json }) => {
      if (!response.ok) {
        return Promise.reject(response);
      }
      if (!json.ok) {
        return Promise.reject(json);
      }
      return json;
    });
}

export default {
  serializeForm,
  urlFor,
  headersFor,
  get,
  post,
  put,
  del,
};
