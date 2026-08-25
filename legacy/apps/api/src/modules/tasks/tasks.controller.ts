import { Body, Controller, Get, Param, ParseIntPipe, Post, Query, UseGuards } from "@nestjs/common";
import { ActorGuard, AllowedActors } from "../../common/actor.js";
import { TasksService } from "./tasks.service.js";
import { ClaimTaskDto, PublishTaskDto } from "./tasks.dto.js";
import type { TaskStatus } from "@buling/shared";

@Controller("tasks")
@UseGuards(ActorGuard)
export class TasksController {
  constructor(private readonly tasks: TasksService) {}

  @Get()
  list(@Query("status") status?: string) {
    const parsed = status?.split(",").filter(Boolean) as TaskStatus[] | undefined;
    return this.tasks.list(parsed).then((data) => ({ data }));
  }

  @Get(":id")
  get(@Param("id") id: string) {
    return this.tasks.get(id).then((data) => ({ data }));
  }

  @Get(":id/intents")
  intents(@Param("id") id: string) {
    return this.tasks.intentsFor(id).then((data) => ({ data }));
  }

  @Post()
  @AllowedActors("user")
  publish(@Body() body: PublishTaskDto) {
    return this.tasks.publish(body).then((data) => ({ data }));
  }

  @Post(":id/claim")
  @AllowedActors("user", "agent")
  claim(@Param("id") id: string, @Body() body: ClaimTaskDto) {
    return this.tasks.claim(id, body).then((data) => ({ data }));
  }

  @Post(":id/pin")
  @AllowedActors("user")
  pin(@Param("id") id: string) {
    return this.tasks.pin(id).then((data) => ({ data }));
  }

  @Post(":id/plan/:idx/complete")
  @AllowedActors("user", "agent")
  complete(@Param("id") id: string, @Param("idx", ParseIntPipe) idx: number) {
    return this.tasks.completeStep(id, idx).then((data) => ({ data }));
  }
}
