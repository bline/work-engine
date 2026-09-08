import { ExperimentalReviewBenchService } from "./service.mjs";

export const REVIEW_BENCH_EXPERIMENTAL_OPERATIONS = Object.freeze(["validate", "compare", "render_case", "inventory"]);

export function createExperimentalReviewBenchAdapter({service=new ExperimentalReviewBenchService()}={}) {
  return async function experimentalReviewBenchAdapter(input={}) {
    if (!REVIEW_BENCH_EXPERIMENTAL_OPERATIONS.includes(input.operation)) throw new TypeError("review-bench experimental operation is invalid");
    const payload=input.payload??{};
    const value=input.operation==="validate"?service.validate(payload):input.operation==="compare"?service.compare(payload):input.operation==="render_case"?service.renderCase(payload):service.inventory(payload);
    return {success:true,contentItems:[{type:"inputText",text:JSON.stringify(value)}]};
  };
}
