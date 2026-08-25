import { Body, Controller, Param, Post, Req, UseGuards } from "@nestjs/common";
import type { FastifyRequest } from "fastify";
import { ActorGuard, AllowedActors, extractActor } from "../../common/actor.js";
import { ReviewService } from "./review.service.js";
import { IsOptional, IsString } from "class-validator";

class RejectDto {
  @IsOptional() @IsString() reason?: string;
}

@Controller("review/docs")
@UseGuards(ActorGuard)
@AllowedActors("user") // 评审红线的第一道栏杆：Guard 拒非人工调用
export class ReviewController {
  constructor(private readonly review: ReviewService) {}

  @Post(":id/approve")
  approve(@Param("id") id: string, @Req() req: FastifyRequest) {
    return this.review.approve(id, extractActor(req)).then((data) => ({ data }));
  }

  @Post(":id/reject")
  reject(@Param("id") id: string, @Body() body: RejectDto, @Req() req: FastifyRequest) {
    return this.review.reject(id, extractActor(req), body.reason).then((data) => ({ data }));
  }
}
