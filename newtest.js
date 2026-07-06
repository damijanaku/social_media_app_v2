

import autocannon from 'autocannon';
import dotenv from 'dotenv'; 

dotenv.config();

const BASE_URL = process.env.BASE_URL || "http://localhost:3000";

const jwttoken = process.env.JWT_TOKEN
const TARGET_USER_ID = process.env.TARGET_USER_ID 
const TARGET_POST_ID = process.env.TARGET_POST_ID 
const TARGET_POST_ID2 = process.env.TARGET_POST_ID2 


const title = "title";
const body = "text";

const jsonHeaders = {
  "content-type": "application/json"
};

const userRequests = [
  { method: "GET", path: "/api/v1/users/profile", headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/username/Jasmine.Bradtke816020315`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/follow/status/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: "/api/v1/users/me/followers", headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: "/api/v1/users/me/following", headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/followers/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/users/following/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
];

const postRequests = [
  { 
    method: "POST", 
    path: "/api/v1/posts", 
    headers: () => ({ ...jsonHeaders, authorization: `Bearer ${jwttoken}` }),
    body: JSON.stringify({ title, body })
  },
  { method: "GET", path: "/api/v1/posts/me", headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: "/api/v1/posts/feed", headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/posts/user/${TARGET_USER_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/posts/${TARGET_POST_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/posts/${TARGET_POST_ID2}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },

];

const likeRequests = [
  { method: "GET", path: `/api/v1/posts/likes/${TARGET_POST_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/posts/likes/${TARGET_POST_ID2}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/posts/likes/check/${TARGET_POST_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/posts/likes/check/${TARGET_POST_ID2}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
];

const commentRequests = [
  {
    method: "POST",
    path: `/api/v1/posts/comments/${TARGET_POST_ID}`,
    headers: () => ({ ...jsonHeaders, authorization: `Bearer ${jwttoken}` }),
    body: JSON.stringify({ title, body })
  },
  {
    method: "POST",
    path: `/api/v1/posts/comments/${TARGET_POST_ID2}`,
    headers: () => ({ ...jsonHeaders, authorization: `Bearer ${jwttoken}` }),
    body: JSON.stringify({ title, body: "Another comment" })
  },
  { method: "GET", path: `/api/v1/posts/comments/${TARGET_POST_ID}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
  { method: "GET", path: `/api/v1/posts/comments/${TARGET_POST_ID2}`, headers: () => ({ authorization: `Bearer ${jwttoken}` }) },
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
  
  add(userRequests, 30);
  add(postRequests, 25);
  add(likeRequests, 20);
  add(commentRequests, 15);
  
  for (let i = pool.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }
  
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