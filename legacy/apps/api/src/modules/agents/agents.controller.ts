import { Controller, Get, Param, UseGuards } from "@nestjs/common";
import { ActorGuard } from "../../common/actor.js";
import { AgentsService } from "./agents.service.js";

@Controller("agents")
@UseGuards(ActorGuard)
export class AgentsController {
  constructor(private readonly agents: AgentsService) {}

  @Get()
  list() {
    return this.agents.list().then((data) => ({ data }));
  }

  @Get(":id")
  get(@Param("id") id: string) {
    return this.agents.get(id).then((data) => ({ data }));
  }
}
