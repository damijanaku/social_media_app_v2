'use strict';

import autocannon from 'autocannon';

const BASE_URL = "http://localhost:3000";

const jwttoken = process.env.JWT_TOKEN;
const TARGET_USER_ID = process.env.TARGET_USER_ID;
const TARGET_POST_ID = process.env.TARGET_POST_ID;
const title = "title";
const body = "text";

const jsonHeaders = {
  "content-type": "application/json"
};


// Requests configuration
const getRequests = [
  { method: "GET", path: "/api/v1/users/profile", headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/posts/user/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: "/api/v1/posts/me", headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: "/api/v1/posts/feed", headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
];

const likeRequests = [
  { method: "GET", path: `/api/v1/posts/likes/${TARGET_POST_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
];

const commentRequests = [
  {
    method: "POST",
    path: `/api/v1/posts/comments/${TARGET_POST_ID}`,
    headers: () => ({ ...jsonHeaders, authorization: `Bearer ${jwttoken}` }),
    body: JSON.stringify({ title, body })
  },
  { method: "GET", path: `/api/v1/posts/comments/${TARGET_POST_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
];

const followRequests = [
  { method: "GET", path: `/api/v1/users/me/followers`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/me/following`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/follow/followers/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/follow/following/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
];

const createPost = [
  {
    method: "POST",
    path: "/api/v1/posts",
    headers: () => ({ ...jsonHeaders, authorization: `Bearer ${jwttoken}` }),
    body: JSON.stringify({ title, body })
  },
];

function buildPool() {
  const pool = [];
  const add = (arr, times) => {
    for (let i = 0; i < times; i++) {
      const request = arr[i % arr.length];
      pool.push({
        ...request,
        headers: typeof request.headers === "function" ? request.headers() : request.headers
      });
    }
  };
  add(getRequests, 50);
  add(likeRequests, 25);
  add(followRequests, 10);
  add(commentRequests, 10);
  add(createPost, 5);
  return pool;
}

(async () => {
  const pool = buildPool();

  const instance = autocannon({
    url: BASE_URL,
    connections: 50,
    duration: 30,
    requests: pool
  },
    (err, result) => {
      if (err) {
        console.error(err);
        process.exit(1);
      }
      autocannon.printResult(result);
    }
  );

  autocannon.track(instance, { renderProgressBar: true });
})();